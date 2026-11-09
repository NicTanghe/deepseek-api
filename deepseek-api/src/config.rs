use crate::{Error, Result};
use reqwest::{header::HeaderMap, Url};
use std::time::Duration;

/// Bounded retries for read-only operations; generation and writes are never replayed.
#[derive(Clone, Debug)]
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
}
impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            initial_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(30),
        }
    }
}
impl RetryPolicy {
    pub(crate) fn delay(&self, attempt: u32) -> Duration {
        self.initial_delay
            .saturating_mul(2u32.saturating_pow(attempt))
            .min(self.max_delay)
    }
}
#[derive(Clone)]
pub(crate) struct Config {
    pub host: Url,
    pub headers: HeaderMap,
    pub retries: RetryPolicy,
}
impl Config {
    pub fn endpoint(&self, path: &str) -> String {
        let base = self.host.as_str().trim_end_matches('/');
        let base = if path.starts_with("v1/") {
            base.strip_suffix("/v1").unwrap_or(base)
        } else {
            base
        };
        let path = if base.ends_with("/beta") {
            path.strip_prefix("beta/").unwrap_or(path)
        } else {
            path
        };
        format!("{base}/{path}")
    }
}
pub(crate) fn validate_host(host: &str) -> Result<Url> {
    let url = Url::parse(host)
        .map_err(|_| Error::Configuration("base URL must be an absolute HTTP(S) URL".into()))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(Error::Configuration("base URL must use HTTP(S), have a host, and contain no credentials, query, or fragment".into()));
    }
    Ok(url)
}
