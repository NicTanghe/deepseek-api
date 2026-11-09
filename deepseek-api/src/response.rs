use serde::{Deserialize, Serialize};
use std::fmt;

/// A model identifier that also accepts future and custom provider models.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ModelId(pub String);
impl ModelId {
    pub const FLASH: &'static str = "deepseek-flash";
    pub const V4_PRO: &'static str = "deepseek-v4-pro";
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}
impl From<&str> for ModelId {
    fn from(id: &str) -> Self {
        Self::new(id)
    }
}
impl From<String> for ModelId {
    fn from(id: String) -> Self {
        Self(id)
    }
}
impl fmt::Display for ModelId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// Convenient model names, with the original identifiers retained for compatibility.
#[derive(Default, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum ModelType {
    #[default]
    #[serde(rename = "deepseek-flash")]
    DeepSeekFlash,
    #[serde(rename = "deepseek-v4-pro")]
    DeepSeekV4Pro,
    #[serde(rename = "deepseek-chat")]
    DeepSeekChat,
    #[serde(rename = "deepseek-reasoner")]
    DeepSeekReasoner,
    #[serde(untagged)]
    Custom(String),
}
impl From<ModelId> for ModelType {
    fn from(id: ModelId) -> Self {
        Self::Custom(id.0)
    }
}
impl From<&str> for ModelType {
    fn from(id: &str) -> Self {
        Self::Custom(id.into())
    }
}
impl From<String> for ModelType {
    fn from(id: String) -> Self {
        Self::Custom(id)
    }
}
impl ModelType {
    /// Historical limits in Ki tokens. Custom model limits must be checked with the provider.
    #[deprecated(
        note = "Model limits vary by provider and mode; consult current API documentation"
    )]
    pub fn get_limit_info(&self) -> (u32, Option<u32>, u32) {
        match self {
            Self::DeepSeekChat => (64, None, 8),
            Self::DeepSeekReasoner => (64, Some(32), 8),
            _ => (0, None, 0),
        }
    }
}
impl fmt::Display for ModelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DeepSeekFlash => "deepseek-flash",
            Self::DeepSeekV4Pro => "deepseek-v4-pro",
            Self::DeepSeekChat => "deepseek-chat",
            Self::DeepSeekReasoner => "deepseek-reasoner",
            Self::Custom(id) => id,
        })
    }
}

/// Represents a model with its associated metadata.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Model {
    /// Unique identifier for the model.
    pub id: String,
    /// Type of the object.
    pub object: String,
    /// Owner of the model.
    pub owned_by: String,
}

/// Response structure containing a list of models.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ModelResp {
    /// Type of the object.
    pub object: String,
    /// List of models.
    pub data: Vec<Model>,
}

/// Information about the balance of a user.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BalanceInfo {
    /// Currency type.
    pub currency: String,
    /// Total balance available.
    pub total_balance: String,
    /// Granted balance.
    pub granted_balance: String,
    /// Topped up balance.
    pub topped_up_balance: String,
}

/// Response structure containing balance information.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BalanceResp {
    /// Indicates if the balance is available.
    pub is_available: bool,
    /// List of balance information.
    pub balance_infos: Vec<BalanceInfo>,
}

/// Represents a function with its name and parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Function {
    /// Name of the function.
    pub name: String,
    /// Arguments of the function, its argument .
    pub arguments: String,
}

/// Represents a tool call with its associated function.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolCall {
    /// Unique identifier for the tool call.
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    /// Function associated with the tool call.
    pub function: Function,
}

/// Represents a message with its content and optional reasoning content and tool calls.
#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct AssistantMessage {
    /// Content of the message.
    #[serde(default, deserialize_with = "null_string")]
    pub content: String,
    /// Optional reasoning content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    /// Optional list of tool calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub prefix: bool,
}

impl AssistantMessage {
    /// Creates a new `AssistantMessage` instance.
    ///
    /// # Arguments
    ///
    /// * `msg` - A string slice representing the message content.
    pub fn new(msg: &str) -> Self {
        AssistantMessage {
            content: msg.to_string(),
            name: None,
            prefix: false,
            reasoning_content: None,
            ..Default::default()
        }
    }

    /// Creates a new `AssistantMessage` instance with a name.
    ///
    /// # Arguments
    ///
    /// * `name` - A string slice representing the name.
    /// * `msg` - A string slice representing the message content.
    pub fn new_with_name(name: &str, msg: &str) -> Self {
        AssistantMessage {
            content: msg.to_string(),
            name: Some(name.to_string()),
            prefix: false,
            reasoning_content: None,
            ..Default::default()
        }
    }

    pub fn set_prefix(mut self, content: &str) -> Self {
        self.prefix = true;
        self.content = content.to_string();
        self
    }
}

/// Enum representing the reason for finishing a process.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FinishReason {
    /// Process stopped.
    #[serde(rename = "stop")]
    Stop,
    /// Process finished due to length limit.
    #[serde(rename = "length")]
    Length,
    /// Process finished due to content filter.
    #[serde(rename = "content_filter")]
    ContentFilter,
    /// Process finished due to tool calls.
    #[serde(rename = "tool_calls")]
    ToolCalls,
    /// Process finished due to insufficient system resources.
    #[serde(rename = "insufficient_system_resource")]
    InsufficientSystemResource,
    #[serde(rename = "aborted")]
    Aborted,
    #[serde(other)]
    Unknown,
}

/// Wrapper for log probability information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogProbWrap {
    /// List of log probabilities.
    #[serde(default, deserialize_with = "null_vec")]
    pub content: Vec<LogProb>,
    /// FIM token log probability metadata.
    pub text_offset: Option<Vec<u32>>,
    pub token_logprobs: Option<Vec<Option<f64>>>,
    pub tokens: Option<Vec<String>>,
    pub top_logprobs: Option<Vec<serde_json::Value>>,
}

/// Represents log probability information for a token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogProb {
    /// Token string.
    pub token: String,
    /// Log probability of the token.
    pub logprob: f32,
    /// Optional bytes representation of the token.
    pub bytes: Option<Vec<u8>>,
    /// List of top log probabilities.
    pub top_logprobs: Vec<TopLogProb>,
}

/// Represents top log probability information for a token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopLogProb {
    /// Token string.
    pub token: String,
    /// Log probability of the token.
    pub logprob: f32,
    /// Optional bytes representation of the token.
    pub bytes: Option<Vec<u8>>,
}

/// Represents a choice made during a process.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Choice {
    /// Reason for finishing the process.
    #[serde(
        default = "unknown_finish_reason",
        deserialize_with = "null_finish_reason"
    )]
    pub finish_reason: FinishReason,
    /// Index of the choice.
    pub index: usize,
    /// Message associated with the choice.
    pub text: Option<String>,
    /// Message associated with the choice.
    pub message: Option<AssistantMessage>,
    /// Optional log probability information.
    pub logprobs: Option<LogProbWrap>,
}

/// Represents usage information for a process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Number of completion tokens used.
    pub completion_tokens: u64,
    /// Number of prompt tokens used.
    pub prompt_tokens: u64,
    /// Number of prompt cache hit tokens.
    #[serde(default)]
    pub prompt_cache_hit_tokens: u64,
    /// Number of prompt cache miss tokens.
    #[serde(default)]
    pub prompt_cache_miss_tokens: u64,
    /// Total number of tokens used.
    pub total_tokens: u64,
    /// Details of completion tokens used.
    pub completion_tokens_details: Option<CompletionTokensDetails>,
    pub prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTokensDetails {
    pub cached_tokens: u64,
}

/// Details of completion tokens used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionTokensDetails {
    /// Number of reasoning tokens used.
    pub reasoning_tokens: u64,
}

/// Represents a chat completion with its associated metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletion {
    /// Unique identifier for the chat completion.
    pub id: String,
    /// List of choices made during the chat completion.
    pub choices: Vec<Choice>,
    /// Timestamp of when the chat completion was created.
    pub created: u32,
    /// Model used for the chat completion.
    pub model: String,
    /// System fingerprint associated with the chat completion.
    pub system_fingerprint: String,
    /// Type of the object.
    pub object: String,
    pub usage: Usage,
}

/// Represents a delta change in a choice stream.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Delta {
    /// Content of the delta change.
    pub content: Option<String>,
    /// Reasoning content of the delta change.
    #[serde(default)]
    pub reasoning_content: Option<String>,
    /// Role of the delta change sender.
    #[serde(default)]
    pub role: String,
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ToolCallDelta {
    pub index: usize,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub tool_type: Option<String>,
    pub function: Option<FunctionDelta>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct FunctionDelta {
    pub name: Option<String>,
    pub arguments: Option<String>,
}

/// Represents a choice stream with its associated delta change.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct JSONChoiceStream {
    /// Delta change in the choice stream.
    pub delta: Delta,
    /// Reason for finishing the choice stream.
    pub finish_reason: Option<FinishReason>,
    /// Index of the choice stream.
    pub index: usize,
}

/// Represents a choice stream with its associated delta change.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TextChoiceStream {
    /// Delta change in the choice stream.
    pub text: String,
    /// Reason for finishing the choice stream.
    pub finish_reason: Option<FinishReason>,
    /// Index of the choice stream.
    pub index: usize,
}

/// Represents a chat completion stream with its associated metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatCompletionStream<T> {
    /// Unique identifier for the chat completion stream.
    pub id: String,
    /// List of choice streams made during the chat completion stream.
    pub choices: Vec<T>,
    /// Timestamp of when the chat completion stream was created.
    pub created: u32,
    /// Model used for the chat completion stream.
    pub model: String,
    /// System fingerprint associated with the chat completion stream.
    pub system_fingerprint: String,
    /// Type of the object.
    pub object: String,
    pub usage: Option<Usage>,
}

#[cfg(not(feature = "is_sync"))]
pub use crate::async_impl::chat_response::ChatResponse;
#[cfg(feature = "is_sync")]
pub use crate::sync_impl::chat_response::ChatResponse;

fn null_string<'de, D: serde::Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}
fn null_vec<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Vec<T>, D::Error> {
    Ok(Option::<Vec<T>>::deserialize(d)?.unwrap_or_default())
}

impl AssistantMessage {
    /// Deserialize a JSON-mode answer into an application type.
    pub fn json<T: serde::de::DeserializeOwned>(&self) -> crate::Result<T> {
        Ok(serde_json::from_str(&self.content)?)
    }
}

fn unknown_finish_reason() -> FinishReason {
    FinishReason::Unknown
}
fn null_finish_reason<'de, D: serde::Deserializer<'de>>(d: D) -> Result<FinishReason, D::Error> {
    Ok(Option::<FinishReason>::deserialize(d)?.unwrap_or(FinishReason::Unknown))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_deserialize_chat_completion_stream() {
        let json_data = json!({
          "id": "6f81a959-b76a-4f5a-94c5-77089b7c646d",
          "object": "chat.completion.chunk",
          "created": 1746341234,
          "model": "deepseek-reasoner",
          "system_fingerprint": "fp_5417b77867_prod0425fp8",
          "choices": [
            {
              "index": 0,
              "delta": {
                "role": "assistant",
                "content": null,
                "reasoning_content": ""
              },
              "logprobs": null,
              "finish_reason": null
            }
          ]
        });

        let deserialized: ChatCompletionStream<JSONChoiceStream> =
            serde_json::from_value(json_data).unwrap();

        assert_eq!(deserialized.id, "6f81a959-b76a-4f5a-94c5-77089b7c646d");
        assert_eq!(deserialized.object, "chat.completion.chunk");
        assert_eq!(deserialized.created, 1746341234);
        assert_eq!(deserialized.model, "deepseek-reasoner");
        assert_eq!(deserialized.system_fingerprint, "fp_5417b77867_prod0425fp8");
        assert_eq!(deserialized.choices.len(), 1);

        let choice = &deserialized.choices[0];
        assert_eq!(choice.index, 0);
        assert_eq!(choice.delta.role, "assistant");
        assert!(choice.delta.content.is_none());
        assert_eq!(choice.delta.reasoning_content.as_ref().unwrap(), "");
        assert!(choice.finish_reason.is_none());
    }
}
