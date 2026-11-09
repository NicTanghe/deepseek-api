use anyhow::Result;
use deepseek_api::{
    asynchronous::DeepSeekClient,
    request::{ContentPart, MessageRequest},
    responses::ResponseRequest,
};
use futures_util::StreamExt;
#[tokio::main]
async fn main() -> Result<()> {
    let client = DeepSeekClient::from_env()?;
    // An optional image URL demonstrates vision input.
    if let Some(url) = std::env::args().nth(1) {
        let messages = [MessageRequest::user_parts(vec![
            ContentPart::text("Describe this image."),
            ContentPart::image(url),
        ])];
        for choice in client.chat(&messages).await?.choices {
            if let Some(message) = choice.message {
                println!("{}", message.content);
            }
        }
    }
    let mut request = ResponseRequest::new("deepseek-flash", "Return JSON with a greeting field.");
    request.options.insert(
        "text".into(),
        serde_json::json!({"format":{"type":"json_object"}}),
    );
    let response = client.responses(&request).await?.into_response()?;
    let value: serde_json::Value = response.json()?;
    println!("{value}");
    request.stream = true;
    let mut events = client.responses(&request).await?.into_stream()?;
    while let Some(event) = events.next().await {
        let event = event?;
        if let Some(text) = event.delta {
            print!("{text}");
        }
        if let Some(response) = event.response {
            println!("\nStatus: {}, usage: {:?}", response.status, response.usage);
        }
    }
    Ok(())
}
