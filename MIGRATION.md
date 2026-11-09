# Migrating from 0.1.1 to 0.2.0-alpha.1

The familiar builders, message types, model variants and `is_async` / `is_sync` feature names remain. This prerelease introduces source and behavior changes and should be tested before upgrading production applications.

- Prefer `asynchronous::DeepSeekClient` or `blocking::DeepSeekClient` and `client.send_completion_request(builder)` for predictable feature unification. Legacy root aliases and builder `do_request()` still select blocking when `is_sync` is enabled.
- Direct client/builder methods now return structured `deepseek_api::Error`. `?` converts it into application `anyhow::Error`; legacy request `do_request()` still returns anyhow. Code that explicitly names an anyhow result may need conversion.
- Empty API keys now fail at client construction. Defaults include a 300-second request timeout, a 10-second connect timeout and two safe read retries. Configure `RetryPolicy { max_retries: 0, ..Default::default() }` to disable retries. Custom HTTP clients retain their transport settings.
- The default model is `deepseek-flash`. Older enum variants preserve their original strings; their availability is provider-defined. New model variants and `Custom` require updates to exhaustive matches. Deprecated `get_limit_info()` contains only historical legacy limits; unknown/modern models return zeroes, meaning unavailable. Do not use it for request validation.
- Added request/response fields mean struct literals may need updating. Prefer constructors/builders; `AssistantMessage::default()` remains available. New `MessageRequest::Multimodal` and `FinishReason` variants also affect exhaustive matches.
- Nullable assistant content deserializes into an empty `String` to preserve existing content access. Use `tool_calls`, `finish_reason`, and reasoning fields to distinguish tool-only/reasoning replies. New streaming tool fragments and usage are optional.
- `Stop` and `ToolChoice` serialize in provider format (string/array/object), fixing the old tagged-enum payloads. Beta chat uses `/beta/chat/completions`; FIM uses `/beta/completions`.
- Chat max tokens accepts 1..=393216; FIM accepts zero as documented. `top_p(0)` is rejected. FIM `logprobs` is now an integer wrapper; the old boolean setter maps true to 1 and false to 0. Use `top_logprobs(count)` for explicit counts. Direct literals using `Option<bool>` must migrate.
- Stream items now carry typed SDK errors. SSE follows event framing (blank-line separators), preserves usage/tool fragments, and reports EOF before a terminal marker as interruption. Replace panic helpers `must_response()` / `must_stream()` with fallible `into_response()?` / `into_stream()?`.
- `Error::Api` replaces the limited legacy `ApiError` mapper for HTTP calls. The old enum remains exported for source compatibility but is no longer emitted. Every non-2xx status is rejected, including redirects and Fetch policy errors.

Responses input/tools/options use JSON values for extensibility. The provider is stateless: do not rely on server-side conversation IDs or arbitrary file/document ingestion. See the compiled examples for RAG, vision, Responses, tools and chat history.

The CLI now uses Ratatui 0.30 and maintained ratatui-textarea, requiring Rust 1.88. The SDK retains Rust 1.85 support.
