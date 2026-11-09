use anyhow::Result;
use clap::Parser;
use deepseek_api::{
    CompletionsRequestBuilder, asynchronous::DeepSeekClientBuilder, request::MessageRequest,
    response::ModelType,
};
use tokio_stream::StreamExt;

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

    let mut stream = client
        .send_completion_request(
            CompletionsRequestBuilder::new(&[MessageRequest::user("how to get to beijing")])
                .use_model(ModelType::DeepSeekFlash)
                .thinking(deepseek_api::request::Thinking::Enabled)
                .stream(true),
        )
        .await?
        .into_stream()?;
    while let Some(item) = stream.next().await {
        println!("resp: {item:?}");
    }
    Ok(())
}
