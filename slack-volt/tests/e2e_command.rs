mod common;

use slack_volt::{AckResponse, CommandContext, Error};
use wiremock::matchers;
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn echo_handler(mut ctx: CommandContext) -> Result<AckResponse, Error> {
    Ok(ctx.ack(format!("echo: {}", ctx.command.text)))
}

#[tokio::test]
async fn test_command_ack_text() {
    let mock_server = MockServer::start().await;

    let app = common::test_app_builder(&mock_server.uri())
        .command("/hello", echo_handler)
        .build();

    let body = common::command_body("/hello", "world", "http://unused");
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await
        .unwrap();

    assert_eq!(result.text.as_deref(), Some("echo: world"));
}

#[tokio::test]
async fn test_command_ack_and_say() {
    let mock_server = MockServer::start().await;
    let _guard = common::mock_post_message(&mock_server).await;

    async fn handler(mut ctx: CommandContext) -> Result<AckResponse, Error> {
        ctx.say("hello channel").await?;
        Ok(ctx.ack("acked"))
    }

    let app = common::test_app_builder(&mock_server.uri())
        .command("/greet", handler)
        .build();

    let body = common::command_body("/greet", "", "http://unused");
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await
        .unwrap();

    assert_eq!(result.text.as_deref(), Some("acked"));
}

#[tokio::test]
async fn test_command_ack_and_respond() {
    let mock_server = MockServer::start().await;

    let _response_mock = Mock::given(matchers::method("POST"))
        .and(matchers::path("/slack/respond"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount_as_scoped(&mock_server)
        .await;

    async fn handler(mut ctx: CommandContext) -> Result<AckResponse, Error> {
        ctx.respond("responded").await?;
        Ok(ctx.ack_empty())
    }

    let response_url = format!("{}/slack/respond", mock_server.uri());
    let app = common::test_app_builder(&mock_server.uri())
        .command("/cmd", handler)
        .build();

    let body = common::command_body("/cmd", "", &response_url);
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await
        .unwrap();

    assert!(result.text.is_none());
}

#[tokio::test]
async fn test_command_ack_and_open_modal() {
    let mock_server = MockServer::start().await;
    let _guard = common::mock_views_open(&mock_server).await;

    async fn handler(mut ctx: CommandContext) -> Result<AckResponse, Error> {
        let view = serde_json::json!({
            "type": "modal",
            "title": {"type": "plain_text", "text": "Test"},
            "blocks": []
        });
        ctx.open_modal(view).await?;
        Ok(ctx.ack_empty())
    }

    let app = common::test_app_builder(&mock_server.uri())
        .command("/modal", handler)
        .build();

    let body = common::command_body("/modal", "", "http://unused");
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await
        .unwrap();

    assert!(result.text.is_none());
}

#[tokio::test]
async fn test_command_ack_ephemeral() {
    let mock_server = MockServer::start().await;

    async fn handler(mut ctx: CommandContext) -> Result<AckResponse, Error> {
        Ok(ctx.ack_ephemeral("only you can see this"))
    }

    let app = common::test_app_builder(&mock_server.uri())
        .command("/secret", handler)
        .build();

    let body = common::command_body("/secret", "", "http://unused");
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await
        .unwrap();

    assert_eq!(result.text.as_deref(), Some("only you can see this"));
    assert_eq!(result.response_type.as_deref(), Some("ephemeral"));
}
