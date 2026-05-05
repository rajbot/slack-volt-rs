mod common;

use slack_volt::{AckResponse, Error, ViewSubmissionContext};
use wiremock::MockServer;

#[tokio::test]
async fn test_view_submission_ack_empty() {
    let mock_server = MockServer::start().await;

    async fn handler(mut ctx: ViewSubmissionContext) -> Result<AckResponse, Error> {
        Ok(ctx.ack())
    }

    let app = common::test_app_builder(&mock_server.uri())
        .view_submission("my_form", handler)
        .build();

    let body = common::view_submission_body("my_form", serde_json::json!({}));
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await
        .unwrap();

    assert!(result.text.is_none());
    assert!(result.blocks.is_none());
}

#[tokio::test]
async fn test_view_submission_ack_errors() {
    let mock_server = MockServer::start().await;

    async fn handler(mut ctx: ViewSubmissionContext) -> Result<AckResponse, Error> {
        Ok(ctx.ack_errors(serde_json::json!({
            "name_block": "Name is required"
        })))
    }

    let app = common::test_app_builder(&mock_server.uri())
        .view_submission("validate_form", handler)
        .build();

    let body = common::view_submission_body("validate_form", serde_json::json!({}));
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await
        .unwrap();

    assert!(result.response_type.is_some());
    let rt = result.response_type.unwrap();
    assert!(rt.contains("errors"));
    assert!(rt.contains("name_block"));
}

#[tokio::test]
async fn test_view_submission_get_value() {
    let mock_server = MockServer::start().await;

    async fn handler(mut ctx: ViewSubmissionContext) -> Result<AckResponse, Error> {
        let val = ctx.get_value("name_block", "name_input");
        assert!(val.is_some());
        let text = val.unwrap()["value"].as_str().unwrap();
        assert_eq!(text, "John");
        Ok(ctx.ack())
    }

    let app = common::test_app_builder(&mock_server.uri())
        .view_submission("data_form", handler)
        .build();

    let values = serde_json::json!({
        "name_block": {
            "name_input": {
                "type": "plain_text_input",
                "value": "John"
            }
        }
    });
    let body = common::view_submission_body("data_form", values);
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await;

    assert!(result.is_ok());
}
