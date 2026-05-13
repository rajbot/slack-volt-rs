use std::sync::Arc;

use crate::request::{SlackAction, SlackCommand, SlackEvent, SlackViewSubmission};
use crate::response::AckResponse;
use crate::Error;

#[derive(Clone)]
pub struct SlackClient {
    token: Arc<str>,
    http: reqwest::Client,
    base_url: Arc<str>,
}

impl std::fmt::Debug for SlackClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlackClient")
            .field("token", &"[REDACTED]")
            .finish()
    }
}

impl SlackClient {
    pub fn new(token: impl Into<Arc<str>>) -> Self {
        Self::with_base_url(token, "https://slack.com/api")
    }

    pub fn with_base_url(token: impl Into<Arc<str>>, base_url: impl Into<Arc<str>>) -> Self {
        Self::with_http(reqwest::Client::new(), token, base_url)
    }

    pub fn with_http(http: reqwest::Client, token: impl Into<Arc<str>>, base_url: impl Into<Arc<str>>) -> Self {
        SlackClient {
            token: token.into(),
            http,
            base_url: base_url.into(),
        }
    }

    pub async fn post_message(
        &self,
        channel: &str,
        text: &str,
    ) -> Result<serde_json::Value, Error> {
        self.api_call(
            "chat.postMessage",
            &serde_json::json!({ "channel": channel, "text": text }),
        )
        .await
    }

    pub async fn post_blocks(
        &self,
        channel: &str,
        blocks: Vec<serde_json::Value>,
        text: &str,
    ) -> Result<serde_json::Value, Error> {
        self.api_call(
            "chat.postMessage",
            &serde_json::json!({
                "channel": channel,
                "blocks": blocks,
                "text": text,
            }),
        )
        .await
    }

    pub async fn open_modal(
        &self,
        trigger_id: &str,
        view: serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        self.api_call(
            "views.open",
            &serde_json::json!({
                "trigger_id": trigger_id,
                "view": view,
            }),
        )
        .await
    }

    pub async fn api_call(
        &self,
        method: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, Error> {
        let url = format!("{}/{method}", self.base_url);
        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.token)
            .json(body)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        if resp["ok"].as_bool() != Some(true) {
            let err = resp["error"].as_str().unwrap_or("unknown error");
            let detail = resp["response_metadata"]["messages"]
                .as_array()
                .map(|msgs| {
                    msgs.iter()
                        .filter_map(|m| m.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            if detail.is_empty() {
                return Err(Error::SlackApi(err.to_string()));
            }
            return Err(Error::SlackApi(format!("{err}: {detail}")));
        }

        Ok(resp)
    }

    pub async fn post_to_url(&self, url: &str, body: &serde_json::Value) -> Result<(), Error> {
        if !is_valid_slack_url(url) {
            return Err(Error::Parse(format!("invalid response_url domain: {url}")));
        }
        self.http.post(url).json(body).send().await?;
        Ok(())
    }
}

pub struct CommandContext {
    pub command: SlackCommand,
    pub client: SlackClient,
    ack_sent: bool,
}

impl CommandContext {
    pub fn new(command: SlackCommand, client: SlackClient) -> Self {
        CommandContext {
            command,
            client,
            ack_sent: false,
        }
    }

    pub fn ack(&mut self, text: impl Into<String>) -> AckResponse {
        self.ack_sent = true;
        AckResponse::text(text)
    }

    pub fn ack_empty(&mut self) -> AckResponse {
        self.ack_sent = true;
        AckResponse::empty()
    }

    pub fn ack_ephemeral(&mut self, text: impl Into<String>) -> AckResponse {
        self.ack_sent = true;
        AckResponse::ephemeral(text)
    }

    pub fn ack_blocks(&mut self, blocks: Vec<serde_json::Value>) -> AckResponse {
        self.ack_sent = true;
        AckResponse::blocks(blocks)
    }

    pub async fn say(&self, text: &str) -> Result<(), Error> {
        self.client
            .post_message(&self.command.channel_id, text)
            .await?;
        Ok(())
    }

    pub async fn respond(&self, text: &str) -> Result<(), Error> {
        self.client
            .post_to_url(&self.command.response_url, &serde_json::json!({ "text": text }))
            .await
    }

    pub async fn open_modal(&self, view: serde_json::Value) -> Result<(), Error> {
        self.client
            .open_modal(&self.command.trigger_id, view)
            .await?;
        Ok(())
    }
}

pub struct EventContext {
    pub event: SlackEvent,
    pub client: SlackClient,
}

impl EventContext {
    pub fn new(event: SlackEvent, client: SlackClient) -> Self {
        EventContext { event, client }
    }
}

pub struct ActionContext {
    pub action: SlackAction,
    pub client: SlackClient,
    ack_sent: bool,
}

impl ActionContext {
    pub fn new(action: SlackAction, client: SlackClient) -> Self {
        ActionContext {
            action,
            client,
            ack_sent: false,
        }
    }

    pub fn ack(&mut self) -> AckResponse {
        self.ack_sent = true;
        AckResponse::empty()
    }

    pub async fn respond(&self, text: &str) -> Result<(), Error> {
        if let Some(ref url) = self.action.response_url {
            self.client
                .post_to_url(url, &serde_json::json!({ "text": text }))
                .await?;
        }
        Ok(())
    }

    pub async fn open_modal(&self, view: serde_json::Value) -> Result<(), Error> {
        self.client
            .open_modal(&self.action.trigger_id, view)
            .await?;
        Ok(())
    }
}

pub struct ViewSubmissionContext {
    pub submission: SlackViewSubmission,
    pub client: SlackClient,
    ack_sent: bool,
}

impl ViewSubmissionContext {
    pub fn new(submission: SlackViewSubmission, client: SlackClient) -> Self {
        ViewSubmissionContext {
            submission,
            client,
            ack_sent: false,
        }
    }

    pub fn ack(&mut self) -> AckResponse {
        self.ack_sent = true;
        AckResponse::empty()
    }

    pub fn ack_errors(&mut self, errors: serde_json::Value) -> AckResponse {
        self.ack_sent = true;
        AckResponse {
            text: None,
            blocks: None,
            response_type: Some(serde_json::to_string(&serde_json::json!({
                "response_action": "errors",
                "errors": errors,
            })).unwrap_or_default()),
        }
    }

    pub fn values(&self) -> Option<&std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>> {
        self.submission.view.state.as_ref().map(|s| &s.values)
    }

    pub fn get_value(&self, block_id: &str, action_id: &str) -> Option<&serde_json::Value> {
        self.values()?.get(block_id)?.get(action_id)
    }

    pub fn private_metadata(&self) -> Option<&str> {
        self.submission.view.private_metadata.as_deref()
    }
}

fn is_valid_slack_url(url: &str) -> bool {
    let Ok(parsed) = url::Url::parse(url) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    if host == "127.0.0.1" || host == "localhost" {
        return true;
    }
    if parsed.scheme() != "https" {
        return false;
    }
    host == "slack.com" || host.ends_with(".slack.com")
}
