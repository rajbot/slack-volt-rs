mod common;

use slack_volt::{AckResponse, ActionContext, CommandContext, Error};
use wiremock::matchers;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_say_returns_slack_api_error() {
    let mock_server = MockServer::start().await;

    Mock::given(matchers::method("POST"))
        .and(matchers::path("/chat.postMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": false,
            "error": "channel_not_found"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    async fn handler(mut ctx: CommandContext) -> Result<AckResponse, Error> {
        ctx.say("hello").await?;
        Ok(ctx.ack("done"))
    }

    let app = common::test_app_builder(&mock_server.uri())
        .command("/test", handler)
        .build();

    let body = common::command_body("/test", "", "http://unused");
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, Error::SlackApi(_)));
    assert!(err.to_string().contains("channel_not_found"));
}

#[tokio::test]
async fn test_say_returns_error_with_response_metadata() {
    let mock_server = MockServer::start().await;

    Mock::given(matchers::method("POST"))
        .and(matchers::path("/chat.postMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": false,
            "error": "invalid_arguments",
            "response_metadata": {
                "messages": ["[ERROR] missing required field: channel"]
            }
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

    async fn handler(mut ctx: CommandContext) -> Result<AckResponse, Error> {
        ctx.say("hello").await?;
        Ok(ctx.ack("done"))
    }

    let app = common::test_app_builder(&mock_server.uri())
        .command("/test", handler)
        .build();

    let body = common::command_body("/test", "", "http://unused");
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("invalid_arguments"));
    assert!(err.to_string().contains("missing required field: channel"));
}

#[tokio::test]
async fn test_open_modal_returns_slack_api_error() {
    let mock_server = MockServer::start().await;

    Mock::given(matchers::method("POST"))
        .and(matchers::path("/views.open"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": false,
            "error": "trigger_expired"
        })))
        .expect(1)
        .mount(&mock_server)
        .await;

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
        .await;

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("trigger_expired"));
}

#[tokio::test]
async fn test_respond_with_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(matchers::method("POST"))
        .and(matchers::path("/slack/respond"))
        .respond_with(ResponseTemplate::new(500))
        .expect(1)
        .mount(&mock_server)
        .await;

    async fn handler(mut ctx: CommandContext) -> Result<AckResponse, Error> {
        ctx.respond("hello").await?;
        Ok(ctx.ack_empty())
    }

    let response_url = format!("{}/slack/respond", mock_server.uri());
    let app = common::test_app_builder(&mock_server.uri())
        .command("/cmd", handler)
        .build();

    let body = common::command_body("/cmd", "", &response_url);
    let headers = common::sign_request(&body);

    // post_to_url doesn't check response status, so this succeeds
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_action_respond_with_no_response_url() {
    let mock_server = MockServer::start().await;

    async fn handler(mut ctx: ActionContext) -> Result<AckResponse, Error> {
        ctx.respond("hello").await?;
        Ok(ctx.ack())
    }

    let app = common::test_app_builder(&mock_server.uri())
        .action("btn", handler)
        .build();

    // action_body with None response_url
    let body = common::action_body("btn", None);
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await;

    // Should succeed silently — no URL to post to
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_say_with_network_error() {
    // Point at a port that's not listening
    async fn handler(mut ctx: CommandContext) -> Result<AckResponse, Error> {
        ctx.say("hello").await?;
        Ok(ctx.ack("done"))
    }

    let app = slack_volt::App::new()
        .signing_secret(common::TEST_SIGNING_SECRET)
        .bot_token(common::TEST_BOT_TOKEN)
        .slack_api_base_url("http://127.0.0.1:1") // nothing listening
        .command("/test", handler)
        .build();

    let body = common::command_body("/test", "", "http://unused");
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::Http(_)));
}
