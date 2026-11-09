use serde::{Deserialize, Serialize};
use std::{fmt, time::Duration};
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ApiErrorBody {
    pub message: String,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub code: Option<serde_json::Value>,
    pub param: Option<serde_json::Value>,
}
/// Structured errors; legacy request builders convert these into anyhow errors.
#[derive(Debug)]
pub enum Error {
    Configuration(String),
    Transport(reqwest::Error),
    Json(serde_json::Error),
    Stream(String),
    Api {
        status: u16,
        error: Box<ApiErrorBody>,
        request_id: Option<String>,
        retry_after: Option<Duration>,
    },
}
impl Error {
    pub fn status(&self) -> Option<u16> {
        if let Self::Api { status, .. } = self {
            Some(*status)
        } else {
            None
        }
    }
    pub fn is_timeout(&self) -> bool {
        matches!(self, Self::Transport(e) if e.is_timeout())
    }
    pub fn is_retryable(&self) -> bool {
        matches!(self.status(), Some(408 | 429 | 500..=599))
            || matches!(self, Self::Transport(e) if e.is_timeout() || e.is_connect())
    }
    pub(crate) fn from_http(
        status: u16,
        headers: &reqwest::header::HeaderMap,
        body: &[u8],
    ) -> Self {
        let error = serde_json::from_slice::<serde_json::Value>(body)
            .ok()
            .and_then(|v| v.get("error").cloned())
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_else(|| ApiErrorBody {
                message: format!("HTTP {status}: response did not contain a JSON API error"),
                ..Default::default()
            });
        Self::Api {
            status,
            error: Box::new(error),
            request_id: headers
                .get("x-request-id")
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned),
            retry_after: headers
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .map(Duration::from_secs),
        }
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Configuration(msg) => write!(f, "invalid configuration: {msg}"),
            Self::Transport(err) => write!(f, "HTTP transport error: {err}"),
            Self::Json(err) => write!(f, "invalid JSON response: {err}"),
            Self::Stream(msg) => write!(f, "stream error: {msg}"),
            Self::Api { status, error, .. } => {
                write!(f, "DeepSeek HTTP {status}: {}", error.message)
            }
        }
    }
}
impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(e) => Some(e),
            Self::Json(e) => Some(e),
            _ => None,
        }
    }
}
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Transport(e.without_url())
    }
}
impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}
