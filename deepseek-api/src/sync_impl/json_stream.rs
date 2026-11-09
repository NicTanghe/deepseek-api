use crate::{sse::Decoder, Error, Result};
use serde::de::DeserializeOwned;
use std::{io::Read, marker::PhantomData};
/// Blocking SSE iterator. Dropping it cancels reading the response body.
pub struct JsonStream<T> {
    response: Box<reqwest::blocking::Response>,
    decoder: Decoder,
    marker: PhantomData<T>,
}
impl<T: DeserializeOwned> JsonStream<T> {
    pub fn new(response: reqwest::blocking::Response) -> Self {
        Self {
            response: Box::new(response),
            decoder: Decoder::default(),
            marker: PhantomData,
        }
    }
}
impl<T: DeserializeOwned> Iterator for JsonStream<T> {
    type Item = Result<T>;
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(event) = self.decoder.pending.pop_front() {
                return Some(event.and_then(|v| serde_json::from_value(v).map_err(Error::from)));
            }
            if self.decoder.done {
                return None;
            }
            let mut buffer = [0; 8192];
            match self.response.read(&mut buffer) {
                Ok(0) => self.decoder.finish(),
                Ok(n) => self.decoder.feed(&buffer[..n]),
                Err(_) => self.decoder.read_error(),
            }
        }
    }
}
impl<T: DeserializeOwned> std::iter::FusedIterator for JsonStream<T> {}
