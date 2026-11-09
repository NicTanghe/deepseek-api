//! Live retrieval APIs for RAG. Retrieved content is untrusted input.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ori_question: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localization: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub need_content: Option<bool>,
}
impl SearchRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            count: None,
            ori_question: None,
            localization: None,
            need_content: None,
        }
    }
    pub fn count(mut self, count: u8) -> Self {
        self.count = Some(count);
        self
    }
    pub fn need_content(mut self, value: bool) -> Self {
        self.need_content = Some(value);
        self
    }
    pub fn validate(&self) -> Result<()> {
        fn text(s: &str) -> bool {
            !s.trim().is_empty() && s.chars().count() <= 128
        }
        if !text(&self.query) || self.ori_question.as_deref().is_some_and(|s| !text(s)) {
            return Err(Error::Configuration("search query/original question must contain 1..=128 Unicode characters and not be blank".into()));
        }
        if self.count.is_some_and(|n| !(1..=50).contains(&n)) {
            return Err(Error::Configuration("search count must be 1..=50".into()));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub rerank_score: f64,
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub summary: String,
    pub content: String,
    pub site_name: Option<String>,
    pub site_icon: Option<String>,
    pub date: Option<String>,
    pub authority: i32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchRequest {
    pub url: String,
}
impl FetchRequest {
    pub fn new(url: impl Into<String>) -> Self {
        Self { url: url.into() }
    }
    pub fn validate(&self) -> Result<()> {
        if self.url.chars().count() > 8192 {
            return Err(Error::Configuration(
                "fetch URL exceeds 8192 characters".into(),
            ));
        }
        let url = reqwest::Url::parse(&self.url)
            .map_err(|_| Error::Configuration("invalid fetch URL".into()))?;
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            return Err(Error::Configuration(
                "fetch requires an HTTP(S) URL with a host and no credentials".into(),
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    pub title: String,
    pub content: String,
}
