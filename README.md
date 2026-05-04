# slack-volt

A Bolt-style framework for building Slack apps in Rust. Provides declarative handler registration, middleware, Block Kit builders, and first-class AWS Lambda support.

Built on top of [slack-morphism](https://github.com/abdolence/slack-morphism-rust) for Slack API types.

## Quick Start

```rust
use slack_volt::{AckResponse, App, CommandContext};
use slack_volt_lambda::LambdaAdapter;

async fn handle_hello(mut ctx: CommandContext) -> Result<AckResponse, slack_volt::Error> {
    Ok(ctx.ack(format!("Hello, <@{}>!", ctx.command.user_id)))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let app = App::new()
        .bot_token(std::env::var("SLACK_BOT_TOKEN")?)
        .signing_secret(std::env::var("SLACK_SIGNING_SECRET")?)
        .command("/hello", handle_hello)
        .build();

    LambdaAdapter::run(app).await?;
    Ok(())
}
```

## Features

- **Slash commands** — `app.command("/name", handler)`
- **Events** — `app.event("app_mention", handler)`
- **Actions** — `app.action("button_click", handler)`
- **Modal submissions** — `app.view_submission("callback_id", handler)`
- **Block Kit DSL** — `blocks::section()`, `blocks::modal()`, `blocks::button()`, etc.
- **Signature verification** — HMAC-SHA256 middleware, enabled automatically when signing secret is set
- **AWS Lambda** — `slack-volt-lambda` crate wraps your app as a Lambda handler via API Gateway

## Crates

| Crate | Description |
|---|---|
| `slack-volt` | Core framework — handlers, middleware, Block Kit |
| `slack-volt-lambda` | AWS Lambda adapter via `lambda_http` |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
slack-volt = { git = "https://github.com/rajbot/slack-volt-rs" }
slack-volt-lambda = { git = "https://github.com/rajbot/slack-volt-rs" }
tokio = { version = "1", features = ["full"] }
```

## Local Development

```bash
# Install cargo-lambda
brew install cargo-lambda

# Run locally (simulates API Gateway)
cargo lambda watch

# Test with curl
curl -X POST http://localhost:9000/lambda-url/hello_command \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "command=%2Fhello&text=world&user_id=U1&channel_id=C1&team_id=T1&trigger_id=tr1&user_name=test&channel_name=dev&response_url=http://example.com"
```

## License

Apache-2.0
