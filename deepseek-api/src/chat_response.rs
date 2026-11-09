use super::json_stream::JsonStream;
use serde::de::DeserializeOwned;
/// Represents a chat response which can either be a full response or a stream of items.
///
/// This enum is generic over the response type `RESP` and the item type `ITEM`.
///
/// # Variants
///
/// - `Full(RESP)`: Represents a complete response of type `RESP`.
/// - `Stream(JsonStream<ITEM>)`: Represents a stream of items of type `ITEM`.
///
/// # Type Parameters
///
/// - `RESP`: The type of the full response. Must implement `DeserializeOwned`.
/// - `ITEM`: The type of the items in the stream. Must implement `DeserializeOwned`.
///
/// # Methods
///
/// - `must_response(self) -> RESP`: Consumes the enum and returns the full response if it is the `Full` variant. Panics if it is the `Stream` variant.
/// - `must_stream(self) -> JsonStream<ITEM>`: Consumes the enum and returns the stream if it is the `Stream` variant. Panics if it is the `Full` variant.
pub enum ChatResponse<RESP, ITEM>
where
    RESP: DeserializeOwned,
    ITEM: DeserializeOwned,
{
    Full(RESP),
    Stream(JsonStream<ITEM>),
}

impl<RESP, ITEM> ChatResponse<RESP, ITEM>
where
    RESP: DeserializeOwned,
    ITEM: DeserializeOwned,
{
    pub fn must_response(self) -> RESP {
        match self {
            ChatResponse::Full(resp) => resp,
            ChatResponse::Stream(_) => panic!("Expected Full variant, found Stream"),
        }
    }

    pub fn must_stream(self) -> JsonStream<ITEM> {
        match self {
            ChatResponse::Stream(stream) => stream,
            ChatResponse::Full(_) => panic!("Expected Stream variant, found Full"),
        }
    }
}

impl<RESP: DeserializeOwned, ITEM: DeserializeOwned> ChatResponse<RESP, ITEM> {
    pub fn into_response(self) -> crate::Result<RESP> {
        match self {
            Self::Full(value) => Ok(value),
            Self::Stream(_) => Err(crate::Error::Configuration(
                "expected a full response".into(),
            )),
        }
    }
    pub fn into_stream(self) -> crate::Result<JsonStream<ITEM>> {
        match self {
            Self::Stream(value) => Ok(value),
            Self::Full(_) => Err(crate::Error::Configuration(
                "expected a streaming response".into(),
            )),
        }
    }
}
