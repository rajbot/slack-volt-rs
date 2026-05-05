mod common;

use slack_volt::{ActionContext, AckResponse, Error};
use wiremock::matchers;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn test_action_ack() {
    let mock_server = MockServer::start().await;

    async fn handler(mut ctx: ActionContext) -> Result<AckResponse, Error> {
        Ok(ctx.ack())
    }

    let app = common::test_app_builder(&mock_server.uri())
        .action("btn_click", handler)
        .build();

    let body = common::action_body("btn_click", None);
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await
        .unwrap();

    assert!(result.text.is_none());
    assert!(result.blocks.is_none());
}

#[tokio::test]
async fn test_action_ack_and_respond() {
    let mock_server = MockServer::start().await;

    let _response_mock = Mock::given(matchers::method("POST"))
        .and(matchers::path("/action/respond"))
        .respond_with(ResponseTemplate::new(200))
        .expect(1)
        .mount_as_scoped(&mock_server)
        .await;

    async fn handler(mut ctx: ActionContext) -> Result<AckResponse, Error> {
        ctx.respond("action response").await?;
        Ok(ctx.ack())
    }

    let response_url = format!("{}/action/respond", mock_server.uri());
    let app = common::test_app_builder(&mock_server.uri())
        .action("btn_respond", handler)
        .build();

    let body = common::action_body("btn_respond", Some(&response_url));
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await
        .unwrap();

    assert!(result.text.is_none());
}

#[tokio::test]
async fn test_action_open_modal() {
    let mock_server = MockServer::start().await;
    let _guard = common::mock_views_open(&mock_server).await;

    async fn handler(mut ctx: ActionContext) -> Result<AckResponse, Error> {
        let view = serde_json::json!({
            "type": "modal",
            "title": {"type": "plain_text", "text": "Action Modal"},
            "blocks": []
        });
        ctx.open_modal(view).await?;
        Ok(ctx.ack())
    }

    let app = common::test_app_builder(&mock_server.uri())
        .action("btn_modal", handler)
        .build();

    let body = common::action_body("btn_modal", None);
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await
        .unwrap();

    assert!(result.text.is_none());
}
