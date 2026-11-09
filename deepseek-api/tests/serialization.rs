use deepseek_api::{
    request::*, response::*, responses::ResponseRequest, search::*, CompletionsRequestBuilder,
    FMICompletionsRequestBuilder, RequestBuilder,
};
use serde_json::{json, Value};
#[test]
fn wire_tool_choices_and_stop_round_trip() {
    let choices = [
        ToolChoice::ChatCompletion(ChatCompletionToolChoice::Required),
        ToolChoice::ChatCompletionNamed(ChatCompletionNamedToolChoice {
            tool_type: ToolType::Function,
            function: FunctionChoice {
                name: "weather".into(),
            },
        }),
    ];
    assert_eq!(
        serde_json::to_value(&choices[0]).unwrap(),
        json!("required")
    );
    assert_eq!(
        serde_json::to_value(&choices[1]).unwrap(),
        json!({"type":"function","function":{"name":"weather"}})
    );
    for choice in choices {
        let wire = serde_json::to_value(choice).unwrap();
        let parsed: ToolChoice = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(parsed).unwrap(), wire);
    }
    assert_eq!(
        serde_json::to_value(Stop::Single("END".into())).unwrap(),
        "END"
    );
}
#[test]
fn modern_models_are_extensible_and_legacy_ids_still_work() {
    for id in [
        "deepseek-flash",
        "deepseek-v4-pro",
        "deepseek-chat",
        "deepseek-reasoner",
        "future-provider-model",
    ] {
        let model: ModelType = serde_json::from_value(json!(id)).unwrap();
        assert_eq!(serde_json::to_value(model).unwrap(), id);
    }
    let messages = [MessageRequest::user("hi")];
    let value = serde_json::to_value(
        CompletionsRequestBuilder::new(&messages)
            .use_model(ModelId::new("custom"))
            .build(),
    )
    .unwrap();
    assert_eq!(value["model"], "custom");
    assert_eq!(
        serde_json::to_value(ModelType::default()).unwrap(),
        "deepseek-flash"
    );
}
#[test]
fn thinking_json_strict_tools_and_omitted_options() {
    let messages = [MessageRequest::sys("Return JSON")];
    let tools = [ToolObject::function(
        "weather",
        "Weather",
        serde_json::from_value(
            json!({"type":"object","properties":{},"additionalProperties":false}),
        )
        .unwrap(),
    )];
    let builder = CompletionsRequestBuilder::new(&messages)
        .thinking(Thinking::Enabled)
        .reasoning_effort(ReasoningEffort::Max)
        .response_format(ResponseType::Json)
        .tools(&tools)
        .strict_tools(true);
    assert_eq!(builder.endpoint(), "beta/chat/completions");
    let value = serde_json::to_value(builder.build()).unwrap();
    assert_eq!(value["tools"][0]["function"]["strict"], true);
    assert_eq!(value["response_format"], json!({"type":"json_object"}));
    assert_eq!(value["thinking"], json!({"type":"enabled"}));
    assert_eq!(value["reasoning_effort"], "max");
    assert!(value.get("max_tokens").is_none());
    assert!(value.get("stream_options").is_none());
}
#[test]
fn multimodal_and_image_file_parts_round_trip() {
    let message = MessageRequest::user_parts(vec![
        ContentPart::text("Describe"),
        ContentPart::image("https://example.com/image.png"),
        ContentPart::File {
            source: FileSource::Id {
                file_id: "file-api-test".into(),
            },
        },
        ContentPart::File {
            source: FileSource::Data {
                file_data: "data:image/png;base64,AA==".into(),
                filename: Some("image.png".into()),
            },
        },
    ]);
    let value = serde_json::to_value(&message).unwrap();
    assert_eq!(value["role"], "user");
    assert_eq!(value["content"][1]["type"], "image_url");
    assert_eq!(
        value["content"][2],
        json!({"type":"file","file_id":"file-api-test"})
    );
    let parsed: MessageRequest = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(parsed.get_content(), "Describe");
    assert_eq!(serde_json::to_value(parsed).unwrap(), value);
    let tool = MessageRequest::tool_parts(
        "call-1",
        vec![ContentPart::image("data:image/png;base64,AA==")],
    );
    let parsed: MessageRequest =
        serde_json::from_value(serde_json::to_value(tool).unwrap()).unwrap();
    assert!(parsed.validate().is_ok());
}
#[test]
fn nullable_assistant_content_usage_and_tool_deltas() {
    let assistant: AssistantMessage =
        serde_json::from_value(json!({"content":null,"reasoning_content":"thinking"})).unwrap();
    assert_eq!(assistant.content, "");
    assert_eq!(assistant.reasoning_content.as_deref(), Some("thinking"));
    let delta:ChatCompletionStream<JSONChoiceStream>=serde_json::from_value(json!({"id":"c","created":1,"model":"deepseek-flash","object":"chat.completion.chunk","system_fingerprint":"f","choices":[{"index":0,"finish_reason":null,"delta":{"tool_calls":[{"index":0,"id":"call","function":{"name":"weather","arguments":"{"}}]}}],"usage":{"prompt_tokens":2,"completion_tokens":3,"total_tokens":5}})).unwrap();
    assert_eq!(delta.usage.unwrap().total_tokens, 5);
    assert_eq!(
        delta.choices[0].delta.tool_calls.as_ref().unwrap()[0]
            .function
            .as_ref()
            .unwrap()
            .arguments
            .as_deref(),
        Some("{")
    );
}
#[test]
fn fim_integer_logprobs_and_echo_checks() {
    let builder = FMICompletionsRequestBuilder::new("prefix", "")
        .max_tokens(0)
        .unwrap()
        .echo(true);
    assert!(builder.validate().is_ok());
    let value = serde_json::to_value(builder.build()).unwrap();
    assert!(value.get("suffix").is_none());
    assert_eq!(value["max_tokens"], 0);
    assert!(FMICompletionsRequestBuilder::new("prefix", "suffix")
        .echo(true)
        .validate()
        .is_err());
    assert_eq!(
        serde_json::to_value(
            FMICompletionsRequestBuilder::new("prefix", "")
                .logprobs(true)
                .build()
        )
        .unwrap()["logprobs"],
        1
    );
    let logs:LogProbWrap=serde_json::from_value(json!({"text_offset":[0],"token_logprobs":[null],"tokens":["a"],"top_logprobs":[{"a":-0.1}]})).unwrap();
    assert_eq!(logs.tokens.unwrap(), vec!["a"]);
}
#[test]
fn query_unicode_boundaries_and_fetch_validation() {
    assert!(SearchRequest::new("x".repeat(128)).validate().is_ok());
    assert!(SearchRequest::new("x".repeat(129)).validate().is_err());
    assert!(SearchRequest::new("\u{1f600}".repeat(128))
        .validate()
        .is_ok());
    assert!(SearchRequest::new("\t\n ").validate().is_err());
    assert!(SearchRequest::new("x").count(0).validate().is_err());
    assert!(SearchRequest::new("x").count(51).validate().is_err());
    let mut request = SearchRequest::new("x");
    request.ori_question = Some(" ".into());
    assert!(request.validate().is_err());
    assert!(
        FetchRequest::new(format!("https://example.com/{}", "x".repeat(8192)))
            .validate()
            .is_err()
    );
    assert!(FetchRequest::new("https://example.com?q=x")
        .validate()
        .is_ok());
}
#[test]
fn numeric_boundaries_reject_nan_and_invalid_ranges() {
    assert!(MaxToken::new(0).is_err());
    assert!(MaxToken::new(393216).is_ok());
    assert!(MaxToken::new(393217).is_err());
    assert!(Temperature::new(f32::NAN).is_err());
    assert!(TopP::new(f32::NAN).is_err());
    assert!(TopP::new(0.0).is_err());
    assert!(TopLogprobs::new(20).is_ok());
    assert!(TopLogprobs::new(21).is_err());
}
#[test]
fn response_options_cannot_duplicate_core_fields_and_json_parsing_is_typed() {
    let mut request = ResponseRequest::new("deepseek-flash", "hi");
    request.options.insert("stream".into(), json!(true));
    assert!(request.validate().is_err());
    let message = AssistantMessage::new(r#"{"value":42}"#);
    assert_eq!(message.json::<Value>().unwrap()["value"], 42);
    assert!(AssistantMessage::new("not JSON").json::<Value>().is_err());
}
#[test]
fn finish_reason_supports_aborted_future_and_null() {
    assert_eq!(
        serde_json::from_value::<FinishReason>(json!("aborted")).unwrap(),
        FinishReason::Aborted
    );
    assert_eq!(
        serde_json::from_value::<FinishReason>(json!("future-reason")).unwrap(),
        FinishReason::Unknown
    );
    let choice: Choice =
        serde_json::from_value(json!({"index":0,"finish_reason":null,"text":"a"})).unwrap();
    assert_eq!(choice.finish_reason, FinishReason::Unknown);
}
#[test]
fn invalid_tools_and_stream_options_fail_validation() {
    let messages = [MessageRequest::user("hi")];
    assert!(CompletionsRequestBuilder::new(&messages)
        .stream_options(StreamOptions::new(true))
        .validate()
        .is_err());
    assert!(CompletionsRequestBuilder::new(&messages)
        .use_model("")
        .validate()
        .is_err());
    assert!(CompletionsRequestBuilder::new(&messages)
        .stop(Stop::Multiple(vec![]))
        .validate()
        .is_err());
    let tool = ToolObject::function("duplicate", "", Default::default());
    let tools = [tool.clone(), tool];
    assert!(CompletionsRequestBuilder::new(&messages)
        .tools(&tools)
        .validate()
        .is_err());
}
