mod support;
use deepseek_api::{
    asynchronous::DeepSeekClientBuilder,
    files::{FileUpload, ListFilesRequest},
    request::*,
    responses::ResponseRequest,
    search::{FetchRequest, SearchRequest},
    CompletionsRequestBuilder, Error, FMICompletionsRequestBuilder, RetryPolicy,
};
use futures_util::StreamExt;
use serde_json::{json, Value};
use std::time::Duration;
use support::{Reply, Server};
fn builder(url: &str) -> DeepSeekClientBuilder {
    DeepSeekClientBuilder::new("test-secret")
        .with_host(url)
        .with_retry_policy(RetryPolicy {
            max_retries: 1,
            initial_delay: Duration::ZERO,
            max_delay: Duration::from_secs(1),
        })
}
const CHAT: &str = r#"{"id":"c","object":"chat.completion","created":1,"model":"deepseek-flash","system_fingerprint":"f","choices":[{"index":0,"finish_reason":"tool_calls","message":{"content":null,"tool_calls":[{"id":"t","type":"function","function":{"name":"weather","arguments":"{}"}}]}}],"usage":{"prompt_tokens":2,"completion_tokens":3,"total_tokens":5}}"#;
const FILE: &str = r#"{"id":"file-api-test","object":"file","bytes":3,"created_at":1,"filename":"test.png","purpose":"user_data"}"#;
#[tokio::test]
async fn chat_payload_auth_and_nullable_tool_reply() {
    let server = Server::new(vec![Reply::json(200, CHAT)]);
    let client = builder(&server.url).build().unwrap();
    let messages = [MessageRequest::user("hello")];
    let response = client
        .send_completion_request(
            CompletionsRequestBuilder::new(&messages)
                .use_model("future-model")
                .thinking(Thinking::Disabled)
                .reasoning_effort(ReasoningEffort::Low)
                .max_tokens(100000)
                .unwrap()
                .stop(Stop::Multiple(vec!["END".into()]))
                .tool_choice(ToolChoice::ChatCompletion(ChatCompletionToolChoice::Auto)),
        )
        .await
        .unwrap()
        .into_response()
        .unwrap();
    assert_eq!(response.choices[0].message.as_ref().unwrap().content, "");
    assert_eq!(
        response.choices[0]
            .message
            .as_ref()
            .unwrap()
            .tool_calls
            .as_ref()
            .unwrap()[0]
            .id,
        "t"
    );
    let requests = server.finish();
    let req = &requests[0];
    assert!(req.head.starts_with("POST /chat/completions "));
    assert!(req
        .head
        .to_lowercase()
        .contains("authorization: bearer test-secret"));
    let payload = req.json();
    assert_eq!(payload["model"], "future-model");
    assert_eq!(payload["thinking"], json!({"type":"disabled"}));
    assert_eq!(payload["reasoning_effort"], "low");
    assert_eq!(payload["stop"], json!(["END"]));
    assert_eq!(payload["tool_choice"], "auto");
    assert_eq!(payload["max_tokens"], 100000);
}
#[tokio::test]
async fn beta_chat_and_fim_use_distinct_routes() {
    let server = Server::new(vec![Reply::json(200, CHAT), Reply::json(200, CHAT)]);
    let client = builder(&server.url).build().unwrap();
    client
        .send_completion_request(
            CompletionsRequestBuilder::new(&[MessageRequest::user("hi")]).use_beta(true),
        )
        .await
        .unwrap();
    client
        .send_completion_request(
            FMICompletionsRequestBuilder::new("prefix", "suffix")
                .top_logprobs(3)
                .unwrap(),
        )
        .await
        .unwrap();
    let requests = server.finish();
    assert!(requests[0].head.starts_with("POST /beta/chat/completions "));
    assert!(requests[1].head.starts_with("POST /beta/completions "));
    assert_eq!(requests[1].json()["logprobs"], 3);
}
#[tokio::test]
async fn retrieval_handles_v1_prefix_nullable_metadata_and_queries() {
    let search = json!({"results":[{"rerank_score":2.4,"title":"Page","url":"https://example.com","snippet":"short","summary":"long","content":"body","site_name":null,"site_icon":null,"date":null,"authority":-1}]});
    let server = Server::new(vec![
        Reply::json(200, search.to_string()),
        Reply::json(200, r##"{"title":"Page","content":"# Hello"}"##),
    ]);
    let client = builder(&format!("{}/v1/", server.url)).build().unwrap();
    let result = client
        .search(
            &SearchRequest::new("cafe site:example.com")
                .count(5)
                .need_content(true),
        )
        .await
        .unwrap();
    assert_eq!(result.results[0].authority, -1);
    assert!(result.results[0].site_name.is_none());
    let result = client
        .fetch(&FetchRequest::new(
            "https://example.com/page?q=value#fragment",
        ))
        .await
        .unwrap();
    assert_eq!(result.content, "# Hello");
    let requests = server.finish();
    assert!(requests[0].head.starts_with("POST /v1/tools/search "));
    assert!(requests[1].head.starts_with("POST /v1/tools/fetch "));
    assert_eq!(requests[0].json()["query"], "cafe site:example.com");
}
#[tokio::test]
async fn safe_reads_retry_429_but_chat_never_retries() {
    let mut limited = Reply::json(
        429,
        r#"{"error":{"message":"slow down","type":"rate_limit","code":"limited"}}"#,
    );
    limited.headers.push(("Retry-After".into(), "0".into()));
    let server = Server::new(vec![limited, Reply::json(200, r#"{"results":[]}"#)]);
    let client = builder(&server.url).build().unwrap();
    assert!(client
        .search(&SearchRequest::new("query"))
        .await
        .unwrap()
        .results
        .is_empty());
    assert_eq!(server.finish().len(), 2);
    let server = Server::new(vec![Reply::json(
        500,
        r#"{"error":{"message":"internal","type":"server_error"}}"#,
    )]);
    let error = builder(&server.url)
        .build()
        .unwrap()
        .chat(&[MessageRequest::user("hi")])
        .await
        .unwrap_err();
    assert_eq!(error.status(), Some(500));
    assert!(error.is_retryable());
    assert_eq!(server.finish().len(), 1);
}
#[tokio::test]
async fn all_error_statuses_are_structured_and_4xx_are_not_retried() {
    for status in [400, 401, 402, 403, 404, 413, 422, 429, 500, 503, 504] {
        let mut reply = Reply::json(
            status,
            r#"{"error":{"message":"denied","type":"invalid_request_error","param":null,"code":"denied"}}"#,
        );
        reply.headers.push(("X-Request-Id".into(), "req-42".into()));
        let server = Server::new(vec![reply]);
        let client = builder(&server.url)
            .with_retry_policy(RetryPolicy {
                max_retries: 0,
                ..Default::default()
            })
            .build()
            .unwrap();
        let error = client.models().await.unwrap_err();
        match error {
            Error::Api {
                status: actual,
                error,
                request_id,
                ..
            } => {
                assert_eq!(actual, status);
                assert_eq!(error.message, "denied");
                assert_eq!(request_id.as_deref(), Some("req-42"));
            }
            e => panic!("unexpected {e:?}"),
        }
        assert_eq!(server.finish().len(), 1);
    }
}
#[tokio::test]
async fn non_json_error_bodies_and_bad_success_json() {
    let server = Server::new(vec![
        Reply::json(404, "<html>secret proxy error</html>"),
        Reply::json(200, "invalid"),
    ]);
    let client = builder(&server.url).build().unwrap();
    let error = client.balance().await.unwrap_err();
    assert_eq!(error.status(), Some(404));
    assert!(!error.to_string().contains("secret proxy error"));
    assert!(matches!(client.models().await.unwrap_err(), Error::Json(_)));
    server.finish();
}
#[tokio::test]
async fn sse_handles_fragmented_utf8_comments_multiline_and_done_fusing() {
    let body = ": ping\r\nevent: chunk\r\nid: 1\r\ndata:{\r\ndata: \"text\":\"caf\u{e9}\"}\r\n\r\ndata:[DONE]\r\n\r\ndata: {\"ignored\":true}\n\n";
    let server = Server::new(vec![Reply::sse(body)]);
    let client = builder(&server.url).build().unwrap();
    let http: Value = json!({"model":"deepseek-flash"});
    // Exercise the public stream with a real chunked HTTP body.
    let response = raw_sse_response(&server.url, &http).await;
    let mut stream = TransportJsonStream::<Value>::new(response);
    assert_eq!(stream.next().await.unwrap().unwrap()["text"], "caf\u{e9}");
    assert!(stream.next().await.is_none());
    assert!(stream.next().await.is_none());
    drop(client);
    server.finish();
}
#[tokio::test]
async fn sse_malformed_event_recovers_and_truncation_is_an_error() {
    let server = Server::new(vec![Reply::sse("data: invalid\n\ndata: {\"ok\":true}\n\n")]);
    let response = raw_sse_response(&server.url, &json!({})).await;
    let mut stream = TransportJsonStream::<Value>::new(response);
    assert!(matches!(stream.next().await.unwrap(), Err(Error::Json(_))));
    assert_eq!(stream.next().await.unwrap().unwrap()["ok"], true);
    assert!(matches!(
        stream.next().await.unwrap(),
        Err(Error::Stream(_))
    ));
    assert!(stream.next().await.is_none());
    server.finish();
}
#[tokio::test]
async fn responses_full_and_semantic_terminal_events() {
    let full = json!({"id":"r","object":"response","created_at":1,"status":"completed","model":"deepseek-flash","output":[{"type":"reasoning","content":[{"type":"reasoning_text","text":"hidden"}]},{"type":"message","content":[{"type":"output_text","text":"answer"}]}],"usage":{"input_tokens":1,"output_tokens":2,"total_tokens":3,"server_tool_use":{"tool_search":1,"tool_fetch":2}}});
    let body = format!("event: response.output_text.delta\ndata: {{\"type\":\"response.output_text.delta\",\"sequence_number\":0,\"delta\":\"answer\"}}\n\nevent: response.completed\ndata: {}\n\n", json!({"type":"response.completed","sequence_number":1,"response":full}));
    let server = Server::new(vec![Reply::json(200, full.to_string()), Reply::sse(body)]);
    let client = builder(&server.url).build().unwrap();
    let mut request = ResponseRequest::new("deepseek-flash", "hello");
    let response = client
        .responses(&request)
        .await
        .unwrap()
        .into_response()
        .unwrap();
    assert_eq!(response.output_text(), "answer");
    assert_eq!(
        response.usage.unwrap().server_tool_use.unwrap().tool_fetch,
        2
    );
    request.stream = true;
    let mut stream = client
        .responses(&request)
        .await
        .unwrap()
        .into_stream()
        .unwrap();
    assert_eq!(
        stream.next().await.unwrap().unwrap().delta.as_deref(),
        Some("answer")
    );
    assert!(stream.next().await.unwrap().unwrap().is_terminal());
    assert!(stream.next().await.is_none());
    let requests = server.finish();
    assert!(requests[0].head.starts_with("POST /responses "));
}
#[tokio::test]
async fn image_files_upload_list_retrieve_delete() {
    let list = json!({"object":"list","data":[serde_json::from_str::<Value>(FILE).unwrap()],"has_more":false,"first_id":"file-api-test","last_id":"file-api-test"});
    let server = Server::new(vec![
        Reply::json(200, FILE),
        Reply::json(200, list.to_string()),
        Reply::json(200, FILE),
        Reply::json(
            200,
            r#"{"id":"file-api-test","object":"file","deleted":true}"#,
        ),
    ]);
    let client = builder(&server.url).build().unwrap();
    let mut upload = FileUpload::new("test.png", vec![1, 2, 3]);
    upload.expires_after_seconds = Some(3600);
    assert_eq!(client.upload_file(upload).await.unwrap().bytes, 3);
    assert!(
        !client
            .list_files(&ListFilesRequest {
                limit: Some(10),
                ..Default::default()
            })
            .await
            .unwrap()
            .has_more
    );
    assert_eq!(
        client
            .retrieve_file("file-api-test")
            .await
            .unwrap()
            .filename,
        "test.png"
    );
    assert!(client.delete_file("file-api-test").await.unwrap().deleted);
    let requests = server.finish();
    let multipart = String::from_utf8_lossy(&requests[0].body);
    assert!(multipart.contains("name=\"purpose\"\r\n\r\nuser_data"));
    assert!(multipart.contains("name=\"expires_after[anchor]\""));
    assert!(multipart.contains("filename=\"test.png\""));
    assert!(requests[1].head.starts_with("GET /files?limit=10 "));
    assert!(requests[2].head.starts_with("GET /files/file-api-test "));
    assert!(requests[3].head.starts_with("DELETE /files/file-api-test "));
}
#[tokio::test]
async fn invalid_inputs_fail_before_network() {
    let client = builder("http://127.0.0.1:1").build().unwrap();
    assert!(matches!(
        client.search(&SearchRequest::new(" ")).await,
        Err(Error::Configuration(_))
    ));
    assert!(client
        .fetch(&FetchRequest::new("file:///secret"))
        .await
        .is_err());
    assert!(client
        .fetch(&FetchRequest::new("https://user:password@example.com"))
        .await
        .is_err());
    assert!(client.chat(&[]).await.is_err());
    assert!(client.retrieve_file("../secret").await.is_err());
}
use deepseek_api::asynchronous::json_stream::JsonStream as TransportJsonStream;
async fn raw_sse_response(url: &str, body: &Value) -> reqwest::Response {
    reqwest::Client::new()
        .post(url)
        .json(body)
        .send()
        .await
        .unwrap()
}

use reqwest::Client as TransportHttpClient;

#[tokio::test]
async fn injected_client_timeout_is_typed_and_no_retry_after_exceeds_policy() {
    let mut reply = Reply::json(200, r#"{"object":"list","data":[]}"#);
    reply.delay = Duration::from_millis(200);
    let server = Server::new(vec![reply]);
    let http = TransportHttpClient::builder()
        .timeout(Duration::from_millis(50))
        .build()
        .unwrap();
    let client = builder(&server.url)
        .with_http_client(http)
        .with_retry_policy(RetryPolicy {
            max_retries: 0,
            ..Default::default()
        })
        .build()
        .unwrap();
    assert!(client.models().await.unwrap_err().is_timeout());
    server.finish();
    let mut reply = Reply::json(429, r#"{"error":{"message":"limited"}}"#);
    reply.headers.push(("Retry-After".into(), "120".into()));
    let server = Server::new(vec![reply]);
    let error = builder(&server.url)
        .build()
        .unwrap()
        .models()
        .await
        .unwrap_err();
    assert_eq!(error.status(), Some(429));
    assert_eq!(server.finish().len(), 1);
}
#[tokio::test]
async fn beta_base_custom_headers_and_transport_errors() {
    let server = Server::new(vec![Reply::json(200, CHAT)]);
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert("x-test", "present".parse().unwrap());
    let client = builder(&format!("{}/beta/", server.url))
        .with_headers(headers)
        .build()
        .unwrap();
    client
        .send_completion_request(CompletionsRequestBuilder::new(&[
            MessageRequest::Assistant(
                deepseek_api::response::AssistantMessage::new("Answer:").set_prefix("Answer:"),
            ),
        ]))
        .await
        .unwrap();
    let requests = server.finish();
    assert!(requests[0].head.starts_with("POST /beta/chat/completions "));
    assert!(requests[0].head.contains("x-test: present"));
    let client = builder("http://127.0.0.1:1")
        .with_retry_policy(RetryPolicy {
            max_retries: 0,
            ..Default::default()
        })
        .build()
        .unwrap();
    let error = client.models().await.unwrap_err();
    assert!(matches!(error, Error::Transport(_)));
    assert!(!format!("{error:?}").contains("test-secret"));
}
