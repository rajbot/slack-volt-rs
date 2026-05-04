use slack_volt::{AckResponse, App, CommandContext};
use slack_volt_lambda::LambdaAdapter;

async fn handle_hello(mut ctx: CommandContext) -> Result<AckResponse, slack_volt::Error> {
    Ok(ctx.ack(format!(
        "Hello, <@{}>! You said: {}",
        ctx.command.user_id, ctx.command.text
    )))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();

    let bot_token = std::env::var("SLACK_BOT_TOKEN").unwrap_or_default();
    let signing_secret = std::env::var("SLACK_SIGNING_SECRET").unwrap_or_default();

    let app = App::new()
        .bot_token(bot_token)
        .signing_secret(signing_secret)
        .command("/hello", handle_hello)
        .build();

    LambdaAdapter::run(app).await?;
    Ok(())
}
