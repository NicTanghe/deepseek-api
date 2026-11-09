use crate::{sse::Decoder, Error, Result};
use futures_util::stream::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
/// SSE JSON stream. Dropping it cancels the HTTP response body.
/// Malformed events yield errors; later correctly framed events remain readable.
pub struct JsonStream<T> {
    inner: Pin<Box<dyn Stream<Item = Result<T>> + Send>>,
}
impl<T: DeserializeOwned + Send + 'static> JsonStream<T> {
    pub fn new(response: reqwest::Response) -> Self {
        let stream = futures_util::stream::unfold(
            (response.bytes_stream(), Decoder::default()),
            |(mut bytes, mut decoder)| async move {
                loop {
                    if let Some(event) = decoder.pending.pop_front() {
                        return Some((
                            event.and_then(|v| serde_json::from_value(v).map_err(Error::from)),
                            (bytes, decoder),
                        ));
                    }
                    if decoder.done {
                        return None;
                    }
                    match bytes.next().await {
                        Some(Ok(bytes)) => decoder.feed(&bytes),
                        Some(Err(_)) => decoder.read_error(),
                        None => decoder.finish(),
                    }
                }
            },
        );
        Self {
            inner: Box::pin(stream.fuse()),
        }
    }
}
impl<T> Stream for JsonStream<T> {
    type Item = Result<T>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}
