pub mod client;
pub mod json_stream;
pub mod chat_response {
    include!("../chat_response.rs");
}
pub mod client_builder {
    include!("../client_builder.rs");
}
use reqwest::blocking::multipart::{Form as MultipartForm, Part as MultipartPart};
use reqwest::blocking::{
    Client as HttpClient, ClientBuilder as HttpClientBuilder, RequestBuilder as HttpRequestBuilder,
    Response as HttpResponse,
};
