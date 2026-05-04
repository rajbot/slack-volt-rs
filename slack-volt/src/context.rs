use crate::request::{SlackAction, SlackCommand, SlackEvent, SlackViewSubmission};
use crate::response::AckResponse;
use crate::Error;

#[derive(Debug, Clone)]
pub struct SlackClient {
    pub token: String,
    http: reqwest::Client,
}

impl SlackClient {
    pub fn new(token: String) -> Self {
        SlackClient {
            token,
            http: reqwest::Client::new(),
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
        let url = format!("https://slack.com/api/{method}");
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
            return Err(Error::SlackApi(err.to_string()));
        }

        Ok(resp)
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
        let http = reqwest::Client::new();
        http.post(&self.command.response_url)
            .json(&serde_json::json!({ "text": text }))
            .send()
            .await?;
        Ok(())
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
            let http = reqwest::Client::new();
            http.post(url)
                .json(&serde_json::json!({ "text": text }))
                .send()
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
