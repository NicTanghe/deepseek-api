# Changelog

## 0.2.0-alpha.1 (unreleased)

- Add explicit async/blocking clients that coexist, validated configuration, injectable HTTP clients, connect/request timeouts, typed API/transport/JSON/stream errors and bounded read retries.
- Add current model names, custom identifiers, thinking controls, vision messages, strict tool schemas, streaming tool fragments and usage, typed JSON parsing, Responses, image file management, Search and Fetch for web RAG.
- Fix stop/tool-choice payloads, beta chat routing, nullable assistant replies, FIM logprobs, token validation and status handling on every endpoint.
- Share SSE framing across transports; support multiline/fragmented data, semantic terminal events, recovery after malformed events, cancellation by drop and interruption errors.
- Recover CLI requests without worker panics or network-held UI locks; preserve reasoning history; add environment configuration, opt-in history, model selection and retrieval/help commands.
- Upgrade CLI to Ratatui 0.30 / maintained ratatui-textarea and remove upstream dependency advisories; retain SDK Rust 1.85 support and require Rust 1.88 for the CLI.
- Refresh examples, migration notes, API reference links, local HTTP/serialization/streaming tests and CI.

This is a prerelease preparation, not a published release. See MIGRATION.md for source/behavior changes.
