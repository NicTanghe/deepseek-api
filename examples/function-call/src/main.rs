use anyhow::{Result, bail};
use clap::Parser;
use deepseek_api::{
    CompletionsRequestBuilder,
    asynchronous::DeepSeekClientBuilder,
    request::{MessageRequest, ToolMessageRequest, ToolObject},
};
use serde::Deserialize;
#[derive(Parser)]
struct Args {
    #[arg(long, env = "DEEPSEEK_API_KEY", hide_env_values = true)]
    api_key: String,
}
#[derive(Deserialize)]
struct WeatherArgs {
    location: String,
}
#[tokio::main]
async fn main() -> Result<()> {
    let client = DeepSeekClientBuilder::new(Args::parse().api_key).build()?;
    let tools = [ToolObject::function(
        "get_weather",
        "Get weather for a location",
        serde_json::from_str(
            r#"{
        "type":"object","properties":{"location":{"type":"string"}},"required":["location"]
    }"#,
        )?,
    )];
    let mut messages = vec![MessageRequest::user("How is the weather in Hangzhou?")];
    // Bound the tool loop so a model cannot call tools indefinitely.
    for _ in 0..4 {
        let response = client
            .send_completion_request(CompletionsRequestBuilder::new(&messages).tools(&tools))
            .await?
            .into_response()?;
        let Some(message) = response.choices.into_iter().find_map(|c| c.message) else {
            bail!("no assistant message returned");
        };
        let calls = message.tool_calls.clone().unwrap_or_default();
        println!("{}", message.content);
        messages.push(MessageRequest::Assistant(message));
        if calls.is_empty() {
            return Ok(());
        }
        for call in calls {
            if call.function.name != "get_weather" {
                bail!("unsupported tool {}", call.function.name);
            }
            let args: WeatherArgs = serde_json::from_str(&call.function.arguments)?;
            // Replace this demo fixture with a real weather service. Validate before execution.
            let output = serde_json::json!({"location":args.location,"temperature_celsius":24,"source":"demo fixture"});
            messages.push(MessageRequest::Tool(ToolMessageRequest::new(
                &output.to_string(),
                &call.id,
            )));
        }
    }
    bail!("tool call limit reached")
}
