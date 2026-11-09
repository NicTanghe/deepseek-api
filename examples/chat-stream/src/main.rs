use anyhow::Result;
use clap::Parser;
use deepseek_api::{blocking::DeepSeekClientBuilder, FMICompletionsRequestBuilder};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(long, env = "DEEPSEEK_API_KEY", hide_env_values = true)]
    pub api_key: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let client = DeepSeekClientBuilder::new(args.api_key.clone()).build()?;

    let balances = client.balance()?;
    println!("balances {balances:?}");

    let models = client.models()?;
    println!("models {models:?}");

    let stream = client
        .send_completion_request(
            FMICompletionsRequestBuilder::new("def fib(a):", "    return fib(a-1) + fib(a-2)")
                .max_tokens(128)?
                .stream(true),
        )?
        .into_stream()?;
    for item in stream {
        println!("resp: {item:?}");
    }

    Ok(())
}
