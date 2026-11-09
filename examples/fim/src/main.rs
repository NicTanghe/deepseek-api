use anyhow::Result;
use clap::Parser;
use deepseek_api::{asynchronous::DeepSeekClientBuilder, FMICompletionsRequestBuilder};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, env = "DEEPSEEK_API_KEY", hide_env_values = true)]
    pub api_key: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let client = DeepSeekClientBuilder::new(args.api_key.clone()).build()?;

    let balances = client.balance().await?;
    println!("balances {balances:?}");

    let models = client.models().await?;
    println!("models {models:?}");

    let resp = client
        .send_completion_request(
            FMICompletionsRequestBuilder::new("def fib(a):", "    return fib(a-1) + fib(a-2)")
                .max_tokens(128)?,
        )
        .await?
        .into_response()?;
    for choice in resp.choices {
        if let Some(text) = choice.text {
            println!("{text}");
        }
    }
    Ok(())
}
