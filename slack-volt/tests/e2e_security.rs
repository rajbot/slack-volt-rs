mod common;

use slack_volt::middleware::Headers;
use slack_volt::{AckResponse, CommandContext, Error};
use std::time::{SystemTime, UNIX_EPOCH};

async fn noop_handler(mut ctx: CommandContext) -> Result<AckResponse, Error> {
    Ok(ctx.ack("hi"))
}

#[tokio::test]
async fn test_bad_signature_rejected() {
    let app = common::test_app_builder("http://unused")
        .command("/hello", noop_handler)
        .build();

    let body = common::command_body("/hello", "text", "http://example.com");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();

    let headers = Headers {
        timestamp: now,
        signature: "v0=deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
            .to_string(),
        content_type: "application/x-www-form-urlencoded".to_string(),
    };
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::SignatureVerification(_)));
}

#[tokio::test]
async fn test_stale_timestamp_rejected() {
    let app = common::test_app_builder("http://unused")
        .command("/hello", noop_handler)
        .build();

    let body = common::command_body("/hello", "text", "http://example.com");
    let stale_ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - 400;

    let headers = common::sign_request_with_timestamp(&body, stale_ts);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::SignatureVerification(_)));
}

#[tokio::test]
async fn test_malformed_json_body() {
    let app = common::test_app_builder("http://unused").build();

    let body = "{{not valid json";
    let headers = common::sign_json_request(body);
    let result = app
        .dispatch_async("application/json", body, headers)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_malformed_form_body() {
    let app = common::test_app_builder("http://unused")
        .command("/hello", noop_handler)
        .build();

    let body = "garbage_no_command_field=true";
    let headers = common::sign_request(body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", body, headers)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_no_handler_registered() {
    let app = common::test_app_builder("http://unused").build();

    let body = common::command_body("/unregistered", "text", "http://example.com");
    let headers = common::sign_request(&body);
    let result = app
        .dispatch_async("application/x-www-form-urlencoded", &body, headers)
        .await;

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), Error::NoHandler { .. }));
}
