# SDK implementation and release notes

## Audit and architecture

The original workspace provided chat/FIM, streaming, model/balance reads and a Ratatui CLI. The saved upgrade plan is local in `ignored/`; that directory is intentionally excluded from Git.

The upgrade retains public builders and compatibility aliases while exposing independent `asynchronous` and `blocking` transports. Builders and response wrappers share source through their transport modules; request/response schemas, configuration, validation, error mapping and SSE framing are shared. Both client implementations exercise equivalent local HTTP fixtures. Async transport remains available with no default features; blocking is opt-in. CLI dependencies stay outside the SDK.

Core fixes include provider-shaped stop/tool-choice payloads, beta chat versus FIM routing, non-2xx rejection on every endpoint, nullable assistant replies, numeric FIM logprobs, updated token validation and current/custom models. Request messages/tools are borrowed by builders; convenience chat accepts a borrowed history slice. Direct calls return typed errors; old `do_request` remains an anyhow adapter.

The HTTP client is reusable/injectable. Standard builders validate keys/base URLs, redact the authorization header, set connect/request timeouts and disable redirects. Retry policy applies only to GET reads and Search/Fetch. Generation, uploads and deletions are not replayed. Raw non-JSON error bodies are not included in errors. The optional `tracing` feature emits method/status/retry diagnostics without request bodies or API keys.

SSE framing is common to both transports. Data may span network chunks/UTF-8 boundaries and multiple `data:` lines. Comments and metadata fields are ignored. JSON event size is capped at 8 MiB. Chat/FIM terminate on `[DONE]`; Responses terminate on semantic completion/incomplete/failure events. Malformed JSON is returned as an item error, allowing subsequent frames to be read. Interrupted streams report an error. Dropping a stream stops reading; async future drop also supports caller cancellation. Blocking cancellation during an active read is limited by HTTP timeout.

## API references verified for this implementation

- [Chat Completions](https://api-docs.deepseek.com/api/create-chat-completion/): current model names, thinking/reasoning effort, tokens, vision parts, tools, usage.
- [FIM](https://api-docs.deepseek.com/api/create-completion/): beta route, numeric logprobs, echo restrictions.
- [Responses reference](https://api-docs.deepseek.com/api/create-response/) and [guide](https://api-docs.deepseek.com/guides/responses_api/): stateless input, semantic streams, server tools, usage.
- [Vision](https://api-docs.deepseek.com/guides/vision/): image URL and file input.
- [Upload](https://api-docs.deepseek.com/api/create-file/), [list](https://api-docs.deepseek.com/api/list-files/), [retrieve](https://api-docs.deepseek.com/api/retrieve-file/), [delete](https://api-docs.deepseek.com/api/delete-file/): multipart image files, expiry and cursor pagination.
- [Search/Fetch guide](https://api-docs.deepseek.com/guides/search/), [Search reference](https://api-docs.deepseek.com/api/search/), [Fetch reference](https://api-docs.deepseek.com/api/fetch/): separate raw JSON retrieval APIs, shared bearer authentication, query syntax, nullable metadata and retryable errors.

Search and Fetch supply live web context for application-owned RAG; no embedding/vector database endpoint is assumed. Files are image storage, not document ingestion. Responses JSON values retain extensible input/tool fields, output items/citations and event data; the SDK does not claim strongly typed coverage of every provider extension. Image availability and model-specific sampling/tool constraints remain provider-controlled.

## Verification and release gates

CI runs local HTTP fixtures and serialization/SSE tests across Linux and Windows, Rust 1.85 for SDK feature combinations and Rust 1.88 for the full workspace; workspace examples, CLI, Clippy and rustdoc checks; dependency advisories and secret scanning. Dependabot covers Cargo and workflow dependencies. Paid live API calls are excluded from tests.

Before publication:

1. Review MIGRATION.md and the prerelease API, especially added enum variants/fields and typed result changes.
2. Run the complete CI matrix and resolve advisory/secret findings. Local Windows checks do not establish Linux success.
3. Run opt-in smoke tests with an owner's API key against the intended account/models, including billing and image support. Never commit keys or chat histories.
4. Confirm SDK and CLI version ownership; they are prepared as 0.2.0-alpha.1. Run package validation, publish the SDK first, wait for registry availability, then publish the CLI.
5. Tag and announce only after publication succeeds. Stable 0.2.0 requires further compatibility review and beta usage.

No publish command runs automatically on push or tag. This implementation does not publish to crates.io, push Git changes or incur live API charges.

## Local verification

Windows validation includes the workspace test suite with both transports, SDK no-default/blocking feature tests (including the Rust 1.85 minimum), compiled documentation examples, warnings-as-errors Clippy/rustdoc, and SDK package verification. Cargo audit against the current advisory database reports no vulnerabilities or informational warnings after upgrading the CLI to Ratatui 0.30 and ratatui-textarea. Linux and GitHub secret scanning are configured in CI and have not been run locally. Live provider smoke tests and registry publication remain release-owner steps.
