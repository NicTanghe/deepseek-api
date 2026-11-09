use super::{
    chat_response::ChatResponse, json_stream::JsonStream, HttpClient, HttpRequestBuilder,
    HttpResponse,
};
use crate::{
    config::Config,
    files::*,
    request::MessageRequest,
    response::{BalanceResp, ChatCompletion, ModelResp},
    responses::{Response, ResponseEvent, ResponseRequest},
    search::*,
    CompletionsRequestBuilder, Error, RequestBuilder, Result,
};
use reqwest::Method;
use serde::{de::DeserializeOwned, Serialize};
/// A reusable authenticated client. Clone it to share the connection pool.
#[derive(Clone)]
pub struct DeepSeekClient {
    pub(crate) client: HttpClient,
    pub(crate) config: Config,
}
impl DeepSeekClient {
    pub fn builder(api_key: impl Into<String>) -> super::client_builder::DeepSeekClientBuilder {
        super::client_builder::DeepSeekClientBuilder::new(api_key)
    }
    pub fn from_env() -> Result<Self> {
        super::client_builder::DeepSeekClientBuilder::default().build()
    }
    fn request(&self, method: Method, path: &str) -> HttpRequestBuilder {
        #[cfg(feature = "tracing")]
        tracing::debug!(method = %method, "sending DeepSeek request");
        self.client
            .request(method, self.config.endpoint(path))
            .headers(self.config.headers.clone())
    }
    fn execute(&self, request: HttpRequestBuilder, retry_read: bool) -> Result<HttpResponse> {
        let mut attempt = 0;
        loop {
            let next = request
                .try_clone()
                .ok_or_else(|| Error::Configuration("request body cannot be replayed".into()))?;
            let result: Result<HttpResponse> = (|| {
                let response = next.send()?;
                if response.status().is_success() {
                    return Ok(response);
                }
                let status = response.status().as_u16();
                let headers = response.headers().clone();
                let body = response.bytes()?;
                Err(Error::from_http(status, &headers, &body))
            })();
            match result {
                Err(error)
                    if retry_read
                        && error.is_retryable()
                        && attempt < self.config.retries.max_retries =>
                {
                    #[cfg(feature = "tracing")]
                    tracing::debug!(
                        attempt,
                        status = error.status(),
                        "retrying read-only request"
                    );
                    let delay = match &error {
                        Error::Api {
                            retry_after: Some(delay),
                            ..
                        } => *delay,
                        _ => self.config.retries.delay(attempt),
                    };
                    // Do not retry earlier than a Retry-After beyond our configured bound.
                    if delay > self.config.retries.max_delay {
                        return Err(error);
                    }
                    std::thread::sleep(delay);
                    attempt += 1;
                }
                result => return result,
            }
        }
    }
    fn json<T: DeserializeOwned>(&self, request: HttpRequestBuilder, retry: bool) -> Result<T> {
        let response = self.execute(request, retry)?;
        Ok(serde_json::from_slice(&response.bytes()?)?)
    }
    pub fn models(&self) -> Result<ModelResp> {
        self.json(self.request(Method::GET, "models"), true)
    }
    pub fn balance(&self) -> Result<BalanceResp> {
        self.json(self.request(Method::GET, "user/balance"), true)
    }
    /// Generate one assistant message. Preserve conversation history in your application.
    pub fn chat(&self, messages: &[MessageRequest]) -> Result<ChatCompletion> {
        self.send_completion_request(CompletionsRequestBuilder::new(messages))?
            .into_response()
    }
    pub fn send_completion_request<B: RequestBuilder>(
        &self,
        builder: B,
    ) -> Result<ChatResponse<B::Response, B::Item>> {
        builder.validate()?;
        let path = builder.endpoint();
        let stream = builder.is_stream();
        let response = self.execute(
            self.request(Method::POST, path).json(&builder.build()),
            false,
        )?;
        if stream {
            Ok(ChatResponse::Stream(JsonStream::new(response)))
        } else {
            Ok(ChatResponse::Full(serde_json::from_slice(
                &response.bytes()?,
            )?))
        }
    }
    pub fn search(&self, request: &SearchRequest) -> Result<SearchResponse> {
        request.validate()?;
        self.json(
            self.request(Method::POST, "v1/tools/search").json(request),
            true,
        )
    }
    pub fn fetch(&self, request: &FetchRequest) -> Result<FetchResponse> {
        request.validate()?;
        self.json(
            self.request(Method::POST, "v1/tools/fetch").json(request),
            true,
        )
    }
    pub fn responses(
        &self,
        request: &ResponseRequest,
    ) -> Result<ChatResponse<Response, ResponseEvent>> {
        request.validate()?;
        let response =
            self.execute(self.request(Method::POST, "responses").json(request), false)?;
        if request.stream {
            Ok(ChatResponse::Stream(JsonStream::new(response)))
        } else {
            Ok(ChatResponse::Full(serde_json::from_slice(
                &response.bytes()?,
            )?))
        }
    }
    pub fn list_files(&self, request: &ListFilesRequest) -> Result<FileList> {
        request.validate()?;
        self.json(self.request(Method::GET, "files").query(request), true)
    }
    pub fn retrieve_file(&self, id: &str) -> Result<FileObject> {
        self.json(self.request(Method::GET, &file_path(id)?), true)
    }
    pub fn delete_file(&self, id: &str) -> Result<FileDeletion> {
        self.json(self.request(Method::DELETE, &file_path(id)?), false)
    }
    /// Upload images only; arbitrary documents are not supported by this provider endpoint.
    pub fn upload_file(&self, upload: FileUpload) -> Result<FileObject> {
        upload.validate()?;
        let part = super::MultipartPart::bytes(upload.bytes).file_name(upload.filename);
        let mut form = super::MultipartForm::new()
            .text("purpose", "user_data")
            .part("file", part);
        if let Some(seconds) = upload.expires_after_seconds {
            form = form
                .text("expires_after[anchor]", "created_at")
                .text("expires_after[seconds]", seconds.to_string());
        }
        // Multipart streams cannot be cloned; send exactly once, with common HTTP error mapping.
        let response = self.request(Method::POST, "files").multipart(form).send()?;
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.bytes()?;
        if !status.is_success() {
            return Err(Error::from_http(status.as_u16(), &headers, &bytes));
        }
        Ok(serde_json::from_slice(&bytes)?)
    }
    /// Provider extension escape hatch, with structured errors and no retries.
    pub fn post_json<T: DeserializeOwned, B: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        if path.starts_with('/') || path.contains("://") || path.split('/').any(|s| s == "..") {
            return Err(Error::Configuration(
                "extension path must be relative to the configured API base".into(),
            ));
        }
        self.json(self.request(Method::POST, path).json(body), false)
    }
}
