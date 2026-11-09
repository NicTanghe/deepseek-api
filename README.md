# DeepSeek API SDK

Rust SDK and terminal chat client for DeepSeek. Includes async and blocking clients, Chat Completions, thinking controls, vision input, tool calls, JSON output, streaming usage, Responses, image files, and web Search/Fetch for RAG.

This branch prepares **0.2.0-alpha.1**. It is a prerelease with migration changes; see [MIGRATION.md](MIGRATION.md) and [CHANGELOG.md](CHANGELOG.md). The SDK requires Rust 1.85 or newer; the CLI/workspace requires Rust 1.88.

## Quick start

For a checkout of this repository:

```powershell
$env:DEEPSEEK_API_KEY = "your-key"
cargo run -p basic
```

To depend on this checkout from another project:

```toml
[dependencies]
deepseek-api = { path = "../deepseek-api/deepseek-api" }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

After the prerelease is published, the path can be replaced with `version = "0.2.0-alpha.1"`.

```rust
use deepseek_api::{asynchronous::DeepSeekClient, request::MessageRequest};

#[tokio::main]
async fn main() -> deepseek_api::Result<()> {
    let client = DeepSeekClient::from_env()?;
    let response = client.chat(&[MessageRequest::user("Hello!")]).await?;
    for choice in response.choices {
        if let Some(message) = choice.message {
            println!("{}", message.content);
        }
    }
    Ok(())
}
```

Conversation state belongs to your application. Retain user and assistant messages and send the full history on each turn. The [basic example](examples/basic/src/main.rs) demonstrates recovery and multi-turn chat.

## Configuration and errors

Use `DeepSeekClient::builder(key)` or `DeepSeekClientBuilder::new(key)`. Builders accept an API key or read `DEEPSEEK_API_KEY` through `Default` / `from_env()`.

Defaults are a 300-second request timeout, a 10-second connect timeout, and two retries for read-only operations. Configure `with_timeout`, `without_timeout`, `with_connect_timeout`, `with_host`, `with_headers`, `with_http_client`, and `with_retry_policy`. An injected HTTP client retains its own timeout, redirect, TLS and proxy configuration. Default transports reject redirects. Empty keys, malformed headers and invalid API base URLs fail locally.

Direct client methods return `deepseek_api::Result<T>`. Match `Error::Api { status, error, request_id, retry_after }`, `Error::Transport`, `Error::Json`, `Error::Stream`, or `Error::Configuration`. `is_timeout()`, `status()` and `is_retryable()` support recovery. Enable the optional `tracing` feature for method/status/retry diagnostics without logging request bodies or credentials. Legacy builder `do_request()` converts into `anyhow::Error`; downcast to `deepseek_api::Error` when necessary.

GET operations and Search/Fetch retry HTTP 408, 429 and 5xx, plus connection failures and timeouts before receiving a response. Retries use bounded exponential delay and honor numeric `Retry-After`; a delay above the configured maximum returns the error instead. Chat, FIM, Responses, uploads and deletions are never automatically replayed. Failures while consuming a body or stream are reported without retrying. Dropping an async request future or either stream cancels further reading; blocking calls run until completion or timeout.

## Models, thinking, tools and JSON

`ModelType::DeepSeekFlash` is the new default; `DeepSeekV4Pro` is also available. Original `DeepSeekChat` and `DeepSeekReasoner` variants retain their wire names. Pass any string or `ModelId` to `.use_model(...)` for future/custom models. Availability remains a server decision.

Use `CompletionsRequestBuilder` for `.thinking(Thinking::Enabled)`, `.reasoning_effort(ReasoningEffort::High)`, sampling, tools and `.response_format(ResponseType::Json)`. Chat token limits accept 1..=393216; effective limits depend on the model/context. In JSON mode, instruct the model to return JSON and parse with `AssistantMessage::json::<T>()`.

`ToolObject::function(...)` constructs function definitions. `.strict_tools(true)` selects the beta chat endpoint and adds `strict` to tool schemas. The [function call example](examples/function-call/src/main.rs) validates arguments and bounds the tool loop. Tools are executed by your application.

## Streaming and Responses

Call `.stream(true)` and `.into_stream()?`, then use `StreamExt::next().await` for async or iterate for blocking. `ChatCompletionStream` includes `usage`, content/reasoning deltas and tool call fragments. Accumulate tool argument strings by index before parsing them. SSE supports comments, CRLF, multiline data, arbitrary byte boundaries, malformed-event errors and terminal detection. Unexpected EOF is an error. Streams stay finished after termination.

`client.responses(&ResponseRequest)` supports full responses and semantic `ResponseEvent`s. Use `response.output_text()` / `.json::<T>()`. Flexible JSON input, tools and options preserve provider extensions. Inspect the terminal response status/error and usage: `response.failed` and `response.incomplete` are terminal events, not transport failures. Responses are stateless; pass prior input/output items back on each turn. `web_search` is a Responses server tool supported by `deepseek-v4-pro`; standalone retrieval uses the separate Search/Fetch methods below.

## Vision and files

Build image messages with `MessageRequest::user_parts` and `ContentPart::text` / `ContentPart::image`. File content parts use `FileSource::Id` or `FileSource::Data`. Tool outputs can also contain image parts. `upload_file`, `list_files`, `retrieve_file`, and `delete_file` handle image storage and pagination, with optional upload expiration. The provider Files API accepts JPEG, PNG, GIF and WebP images; it does not ingest arbitrary documents or create a vector database. Upload size is limited to 64 MiB. See the [modern example](examples/modern/src/main.rs).

## Search, Fetch and RAG

```rust
use deepseek_api::search::{SearchRequest, FetchRequest};
# async fn retrieve(client: &deepseek_api::asynchronous::DeepSeekClient) -> deepseek_api::Result<()> {
let results = client.search(&SearchRequest::new("DeepSeek site:github.com")
    .count(5).need_content(true)).await?;
for result in results.results {
    println!("{}: {}", result.title, result.url);
}
let page = client.fetch(&FetchRequest::new("https://example.com")).await?;
println!("{}", page.content);
# Ok(()) }
```

Search calls `/v1/tools/search`; Fetch calls `/v1/tools/fetch`, using the same bearer key. Query operators are passed through unchanged. Search validates Unicode query length (1..=128) and count (1..=50); nullable source metadata and empty results are preserved. Fetch validates HTTP(S) URLs, the 8192-character limit and absence of credentials. Both root and `/v1` API base URLs work.

Run the [RAG example](examples/rag/src/main.rs): `cargo run -p rag -- "your question"`. It retrieves sources, falls back to Fetch, bounds source text, and asks for URL citations. Retrieved pages are untrusted data and may contain prompt injection. Search/Fetch incur separate provider charges; consult [DeepSeek's current guide](https://api-docs.deepseek.com/guides/search/) for billing and rate limits.

## Blocking and feature combinations

```toml
deepseek-api = { path = "../deepseek-api/deepseek-api", default-features = false, features = ["blocking"] }
```

Use `deepseek_api::blocking::DeepSeekClient` with the same methods, without `.await`. Async types remain available under `deepseek_api::asynchronous`. `is_async` / `is_sync` remain compatibility feature names; `async` / `blocking` are aliases. With `is_sync`, legacy root client/stream aliases and builder `do_request()` select blocking. Explicit namespaces work together regardless of feature unification. No default features still supports async transport.

## Terminal CLI

```powershell
cargo run -p ds-cli -- --model deepseek-flash
cargo run -p ds-cli -- --history ./chat-history.json
```

Configuration: `DEEPSEEK_API_KEY`, `DEEPSEEK_MODEL`, `DEEPSEEK_BASE_URL`, or the corresponding flags. History is saved only when `--history` is supplied; that file contains conversation and reasoning text. Commands: `/help`, `/models`, `/balance`, `/model <id>`, `/clear`, `/search <query>`, `/fetch <url>`, `/exit`. Search/Fetch add retrieved context to the current session. Tab completes unique slash commands or moves focus, Enter activates/sends, Ctrl+J inserts a newline, Esc exits. The Deep Think button toggles thinking on the selected model. Request failures leave the UI usable and remove incomplete turns from API history.

## Development

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test -p deepseek-api --no-default-features
cargo test -p deepseek-api --no-default-features --features blocking
```

Tests use local HTTP servers and require no credentials or paid calls. CI checks Linux/Windows, the feature matrix, documentation, dependency advisories and secrets. See [the implementation notes](docs/implementation.md) for API references and release requirements.
