use anyhow::Result;
use clap::Parser;
use deepseek_api::{asynchronous::DeepSeekClientBuilder, request::MessageRequest};
use std::io::{stdin, stdout, Write};
#[derive(Parser)]
struct Args {
    #[arg(long, env = "DEEPSEEK_API_KEY", hide_env_values = true)]
    api_key: String,
}
#[tokio::main]
async fn main() -> Result<()> {
    let client = DeepSeekClientBuilder::new(Args::parse().api_key).build()?;
    let mut history = Vec::new();
    loop {
        print!("> ");
        stdout().flush()?;
        let mut input = String::new();
        if stdin().read_line(&mut input)? == 0 {
            break;
        }
        match input.trim() {
            "" => continue,
            "/exit" => break,
            "/clear" => {
                history.clear();
                continue;
            }
            "/models" => {
                println!("{:?}", client.models().await?);
                continue;
            }
            "/balance" => {
                println!("{:?}", client.balance().await?);
                continue;
            }
            question => history.push(MessageRequest::user(question)),
        }
        match client.chat(&history).await {
            Ok(response) => {
                for choice in response.choices {
                    if let Some(message) = choice.message {
                        println!("{}", message.content);
                        history.push(MessageRequest::Assistant(message));
                    }
                }
            }
            Err(error) => {
                history.pop();
                eprintln!("{error}");
            }
        }
    }
    Ok(())
}
