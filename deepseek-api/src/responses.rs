//! Stateless Responses API. Send prior input/output items again for each turn.
use crate::{request::ReasoningEffort, response::ModelType, Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
/// Flexible items/tools preserve new provider fields without an SDK release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseRequest {
    pub model: ModelType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ResponseReasoning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    pub stream: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tools: Vec<Value>,
    #[serde(flatten)]
    pub options: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseReasoning {
    pub effort: ReasoningEffort,
}
impl ResponseRequest {
    pub fn new(model: impl Into<ModelType>, input: impl Into<Value>) -> Self {
        Self {
            model: model.into(),
            input: Some(input.into()),
            instructions: None,
            reasoning: None,
            max_output_tokens: None,
            stream: false,
            tools: Vec::new(),
            options: BTreeMap::new(),
        }
    }
    pub fn validate(&self) -> Result<()> {
        if self.input.as_ref().is_none_or(Value::is_null) && self.instructions.is_none() {
            return Err(Error::Configuration(
                "Responses requires input or instructions".into(),
            ));
        }
        if self.max_output_tokens == Some(0) {
            return Err(Error::Configuration(
                "max_output_tokens must be positive".into(),
            ));
        }
        for key in [
            "model",
            "input",
            "instructions",
            "reasoning",
            "max_output_tokens",
            "stream",
            "tools",
        ] {
            if self.options.contains_key(key) {
                return Err(Error::Configuration(format!(
                    "duplicate Responses option: {key}"
                )));
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub id: String,
    pub object: String,
    pub created_at: u64,
    pub status: String,
    pub model: String,
    pub output: Vec<Value>,
    pub usage: Option<ResponseUsage>,
    pub error: Option<Value>,
    pub incomplete_details: Option<Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
impl Response {
    pub fn output_text(&self) -> String {
        self.output
            .iter()
            .filter(|v| v["type"] == "message")
            .filter_map(|v| v["content"].as_array())
            .flatten()
            .filter(|v| v["type"] == "output_text")
            .filter_map(|v| v["text"].as_str())
            .collect()
    }
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_str(&self.output_text())?)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub input_tokens_details: Option<Value>,
    pub output_tokens_details: Option<Value>,
    pub server_tool_use: Option<ServerToolUse>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerToolUse {
    pub tool_search: u64,
    pub tool_fetch: u64,
}
/// Semantic SSE event, retaining all provider fields (tool arguments, citations, items, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub sequence_number: Option<u64>,
    pub delta: Option<String>,
    pub response: Option<Response>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
impl ResponseEvent {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.event_type.as_str(),
            "response.completed" | "response.incomplete" | "response.failed"
        )
    }
}
