use super::client::DeepSeekClient;
use super::{HttpClient, HttpClientBuilder};
use crate::{
    config::{validate_host, Config},
    Error, Result, RetryPolicy,
};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use std::{env, time::Duration};
/// Configure authentication, transport, timeouts and safe read retries.
/// Injected HTTP clients keep their own timeout, TLS, proxy and redirect settings.
pub struct DeepSeekClientBuilder {
    api_key: String,
    timeout: Option<Duration>,
    connect_timeout: Duration,
    host: String,
    headers: HeaderMap,
    retries: RetryPolicy,
    client: Option<HttpClient>,
}
impl Default for DeepSeekClientBuilder {
    fn default() -> Self {
        Self::new(env::var("DEEPSEEK_API_KEY").unwrap_or_default())
    }
}
impl DeepSeekClientBuilder {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            timeout: Some(Duration::from_secs(300)),
            connect_timeout: Duration::from_secs(10),
            host: "https://api.deepseek.com".into(),
            headers: HeaderMap::new(),
            retries: RetryPolicy::default(),
            client: None,
        }
    }
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout = Some(Duration::from_secs(seconds));
        self
    }
    pub fn without_timeout(mut self) -> Self {
        self.timeout = None;
        self
    }
    pub fn with_connect_timeout(mut self, seconds: u64) -> Self {
        self.connect_timeout = Duration::from_secs(seconds);
        self
    }
    pub fn with_host(mut self, host: &str) -> Self {
        self.host = host.into();
        self
    }
    pub fn with_api_key(mut self, key: &str) -> Self {
        self.api_key = key.into();
        self
    }
    pub fn with_headers(mut self, headers: HeaderMap) -> Self {
        self.headers = headers;
        self
    }
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.retries = policy;
        self
    }
    pub fn with_http_client(mut self, client: HttpClient) -> Self {
        self.client = Some(client);
        self
    }
    pub fn build(self) -> Result<DeepSeekClient> {
        if self.api_key.trim().is_empty() {
            return Err(Error::Configuration(
                "API key is empty; set DEEPSEEK_API_KEY or supply a key".into(),
            ));
        }
        let host = validate_host(&self.host)?;
        let mut auth =
            HeaderValue::from_str(&format!("Bearer {}", self.api_key)).map_err(|_| {
                Error::Configuration("API key contains invalid header characters".into())
            })?;
        auth.set_sensitive(true);
        let mut headers = self.headers;
        headers.insert(AUTHORIZATION, auth);
        let client = match self.client {
            Some(client) => client,
            None => {
                let builder = HttpClientBuilder::new()
                    .connect_timeout(self.connect_timeout)
                    .redirect(reqwest::redirect::Policy::none());
                let builder = if let Some(timeout) = self.timeout {
                    builder.timeout(timeout)
                } else {
                    builder
                };
                builder.build()?
            }
        };
        Ok(DeepSeekClient {
            client,
            config: Config {
                host,
                headers,
                retries: self.retries,
            },
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_configuration() {
        assert!(DeepSeekClientBuilder::new("").build().is_err());
        for host in [
            "ftp://example.com",
            "https://user:password@example.com",
            "not a url",
            "https://example.com?key=secret",
        ] {
            assert!(DeepSeekClientBuilder::new("test")
                .with_host(host)
                .build()
                .is_err());
        }
        assert!(DeepSeekClientBuilder::new("bad\nkey").build().is_err());
    }
}
