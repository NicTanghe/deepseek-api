use anyhow::Result;
use clap::Parser;
use deepseek_api::{blocking::DeepSeekClientBuilder, request::MessageRequest};
#[derive(Parser)]
struct Args {
    #[arg(long, env = "DEEPSEEK_API_KEY", hide_env_values = true)]
    api_key: String,
}
fn main() -> Result<()> {
    let client = DeepSeekClientBuilder::new(Args::parse().api_key).build()?;
    let mut history = vec![MessageRequest::user("Hello!")];
    for choice in client.chat(&history)?.choices {
        if let Some(message) = choice.message {
            println!("{}", message.content);
            history.push(MessageRequest::Assistant(message));
        }
    }
    Ok(())
}
