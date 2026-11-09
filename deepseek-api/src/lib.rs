//! DeepSeek SDK: chat, vision, streaming, Responses, image files and web retrieval.
//! Async is always available; enable `blocking` (or legacy `is_sync`) for the blocking client.
//! With `is_sync`, the legacy root aliases select blocking; explicit namespaces coexist.
//!
//! ```no_run
//! use deepseek_api::{asynchronous::DeepSeekClient, request::MessageRequest};
//! # async fn example() -> deepseek_api::Result<()> {
//! let client = DeepSeekClient::from_env()?;
//! let completion = client.chat(&[MessageRequest::user("Hello!")]).await?;
//! for choice in completion.choices {
//!     if let Some(message) = choice.message { println!("{}", message.content); }
//! }
//! # Ok(()) }
//! ```
//!
//! ```no_run
//! use deepseek_api::{asynchronous::DeepSeekClient, request::{MessageRequest, Thinking}, CompletionsRequestBuilder};
//! use futures_util::StreamExt;
//! # async fn stream() -> deepseek_api::Result<()> {
//! let client = DeepSeekClient::from_env()?;
//! let messages = [MessageRequest::user("Explain Rust lifetimes")];
//! let mut stream = client.send_completion_request(CompletionsRequestBuilder::new(&messages)
//!     .thinking(Thinking::Enabled).stream(true)).await?.into_stream()?;
//! while let Some(chunk) = stream.next().await { println!("{:?}", chunk?); }
//! # Ok(()) }
//! ```
//!
//! ```no_run
//! use deepseek_api::{asynchronous::DeepSeekClient, search::{SearchRequest, FetchRequest}};
//! # async fn retrieve() -> deepseek_api::Result<()> {
//! let client = DeepSeekClient::from_env()?;
//! let results = client.search(&SearchRequest::new("DeepSeek").count(5)).await?;
//! for result in results.results { println!("{}", result.url); }
//! let page = client.fetch(&FetchRequest::new("https://example.com")).await?;
//! # Ok(()) }
//! ```
//!
//! ```no_run
//! use deepseek_api::{asynchronous::DeepSeekClient, responses::ResponseRequest};
//! # async fn responses() -> deepseek_api::Result<()> {
//! let client = DeepSeekClient::from_env()?;
//! let response = client.responses(&ResponseRequest::new("deepseek-flash", "Hello")).await?.into_response()?;
//! println!("{}", response.output_text());
//! # Ok(()) }
//! ```
mod config;
mod error;
pub mod files;
pub mod request;
mod request_builder;
pub mod response;
pub mod responses;
pub mod search;
mod sse;
mod typed_error;
pub use config::RetryPolicy;
pub use error::ApiError;
pub use request_builder::*;
pub use typed_error::*;
mod async_impl;
#[cfg(feature = "is_sync")]
mod sync_impl;
/// Explicit async types, unaffected by feature unification.
pub mod asynchronous {
    pub use crate::async_impl::{
        chat_response::ChatResponse, client::*, client_builder::*, json_stream,
    };
}
/// Explicit blocking types. Enable the `blocking` or `is_sync` feature.
#[cfg(feature = "is_sync")]
pub mod blocking {
    pub use crate::sync_impl::{
        chat_response::ChatResponse, client::*, client_builder::*, json_stream,
    };
}
#[cfg(not(feature = "is_sync"))]
pub use asynchronous::{json_stream, DeepSeekClient, DeepSeekClientBuilder};
#[cfg(feature = "is_sync")]
pub use blocking::{json_stream, DeepSeekClient, DeepSeekClientBuilder};
