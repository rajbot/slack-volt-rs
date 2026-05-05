mod common;

use slack_volt::{AckResponse, Error, EventContext};
use wiremock::MockServer;

#[tokio::test]
async fn test_event_app_mention() {
    let mock_server = MockServer::start().await;

    async fn mention_handler(ctx: EventContext) -> Result<AckResponse, Error> {
        assert_eq!(ctx.event.event_type, "app_mention");
        Ok(AckResponse::empty())
    }

    let app = common::test_app_builder(&mock_server.uri())
        .event("app_mention", mention_handler)
        .build();

    let body = common::event_body("app_mention", serde_json::json!({"text": "hi", "user": "U1"}));
    let headers = common::sign_json_request(&body);
    let result = app
        .dispatch_async("application/json", &body, headers)
        .await
        .unwrap();

    assert!(result.text.is_none());
    assert!(result.blocks.is_none());
}

#[tokio::test]
async fn test_event_message() {
    let mock_server = MockServer::start().await;

    async fn message_handler(_ctx: EventContext) -> Result<AckResponse, Error> {
        Ok(AckResponse::empty())
    }

    let app = common::test_app_builder(&mock_server.uri())
        .event("message", message_handler)
        .build();

    let body = common::event_body(
        "message",
        serde_json::json!({"text": "hello", "user": "U1", "channel": "C1"}),
    );
    let headers = common::sign_json_request(&body);
    let result = app
        .dispatch_async("application/json", &body, headers)
        .await
        .unwrap();

    assert!(result.text.is_none());
}

#[tokio::test]
async fn test_event_with_say() {
    let mock_server = MockServer::start().await;
    let _guard = common::mock_post_message(&mock_server).await;

    async fn handler(ctx: EventContext) -> Result<AckResponse, Error> {
        ctx.client.post_message("C_TEST", "replying").await?;
        Ok(AckResponse::empty())
    }

    let app = common::test_app_builder(&mock_server.uri())
        .event("app_mention", handler)
        .build();

    let body = common::event_body("app_mention", serde_json::json!({"text": "hi", "user": "U1"}));
    let headers = common::sign_json_request(&body);
    let result = app
        .dispatch_async("application/json", &body, headers)
        .await
        .unwrap();

    assert!(result.text.is_none());
}
