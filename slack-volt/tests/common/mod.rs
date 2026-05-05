#![allow(dead_code)]

use hmac::{Hmac, Mac};
use sha2::Sha256;
use slack_volt::middleware::Headers;
use std::time::{SystemTime, UNIX_EPOCH};
use wiremock::matchers;
use wiremock::{Mock, MockServer, ResponseTemplate};

pub const TEST_SIGNING_SECRET: &str = "test_signing_secret_e2e";
pub const TEST_BOT_TOKEN: &str = "xoxb-test-token-e2e";

pub fn sign_request(body: &str) -> Headers {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .to_string();
    let basestring = format!("v0:{now}:{body}");
    let mut mac = Hmac::<Sha256>::new_from_slice(TEST_SIGNING_SECRET.as_bytes()).unwrap();
    mac.update(basestring.as_bytes());
    let sig = format!("v0={}", hex::encode(mac.finalize().into_bytes()));
    Headers {
        timestamp: now,
        signature: sig,
        content_type: "application/x-www-form-urlencoded".to_string(),
    }
}

pub fn sign_json_request(body: &str) -> Headers {
    let mut h = sign_request(body);
    h.content_type = "application/json".to_string();
    h
}

pub fn sign_request_with_timestamp(body: &str, timestamp: u64) -> Headers {
    let ts = timestamp.to_string();
    let basestring = format!("v0:{ts}:{body}");
    let mut mac = Hmac::<Sha256>::new_from_slice(TEST_SIGNING_SECRET.as_bytes()).unwrap();
    mac.update(basestring.as_bytes());
    let sig = format!("v0={}", hex::encode(mac.finalize().into_bytes()));
    Headers {
        timestamp: ts,
        signature: sig,
        content_type: "application/x-www-form-urlencoded".to_string(),
    }
}

pub fn command_body(command: &str, text: &str, response_url: &str) -> String {
    serde_urlencoded::to_string(&[
        ("command", command),
        ("text", text),
        ("user_id", "U_TEST"),
        ("channel_id", "C_TEST"),
        ("team_id", "T_TEST"),
        ("trigger_id", "tr_test"),
        ("user_name", "testuser"),
        ("channel_name", "general"),
        ("response_url", response_url),
    ])
    .unwrap()
}

pub fn action_body(action_id: &str, response_url: Option<&str>) -> String {
    let mut payload = serde_json::json!({
        "type": "block_actions",
        "trigger_id": "tr_test",
        "user": {"id": "U_TEST"},
        "actions": [{"action_id": action_id, "type": "button"}]
    });
    if let Some(url) = response_url {
        payload["response_url"] = serde_json::Value::String(url.to_string());
    }
    serde_urlencoded::to_string(&[("payload", serde_json::to_string(&payload).unwrap())]).unwrap()
}

pub fn view_submission_body(callback_id: &str, values: serde_json::Value) -> String {
    let payload = serde_json::json!({
        "type": "view_submission",
        "trigger_id": "tr_test",
        "user": {"id": "U_TEST"},
        "view": {
            "id": "V_TEST",
            "callback_id": callback_id,
            "state": {"values": values},
            "private_metadata": ""
        }
    });
    serde_urlencoded::to_string(&[("payload", serde_json::to_string(&payload).unwrap())]).unwrap()
}

pub fn event_body(event_type: &str, extra: serde_json::Value) -> String {
    let mut event = extra;
    event["type"] = serde_json::Value::String(event_type.to_string());
    serde_json::to_string(&serde_json::json!({
        "team_id": "T_TEST",
        "event_id": "Ev_TEST",
        "event": event
    }))
    .unwrap()
}

pub async fn mock_post_message(server: &MockServer) -> wiremock::MockGuard {
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/chat.postMessage"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})),
        )
        .named("chat.postMessage")
        .expect(1)
        .mount_as_scoped(server)
        .await
}

pub async fn mock_views_open(server: &MockServer) -> wiremock::MockGuard {
    Mock::given(matchers::method("POST"))
        .and(matchers::path("/views.open"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true})),
        )
        .named("views.open")
        .expect(1)
        .mount_as_scoped(server)
        .await
}

pub fn test_app_builder(mock_server_uri: &str) -> slack_volt::AppBuilder {
    slack_volt::App::new()
        .signing_secret(TEST_SIGNING_SECRET)
        .bot_token(TEST_BOT_TOKEN)
        .slack_api_base_url(mock_server_uri)
}
