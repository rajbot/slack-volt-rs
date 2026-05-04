use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum SlackRequest {
    Command(SlackCommand),
    Event(SlackEvent),
    Action(SlackAction),
    ViewSubmission(SlackViewSubmission),
    UrlVerification { challenge: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackCommand {
    pub command: String,
    pub text: String,
    pub trigger_id: String,
    pub user_id: String,
    pub user_name: String,
    pub channel_id: String,
    pub channel_name: String,
    pub team_id: String,
    pub response_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackEvent {
    pub event_type: String,
    pub team_id: String,
    pub event: serde_json::Value,
    pub event_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackAction {
    pub action_id: String,
    pub trigger_id: String,
    pub user: SlackUser,
    pub channel: Option<SlackChannel>,
    pub response_url: Option<String>,
    pub actions: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackViewSubmission {
    pub callback_id: String,
    pub trigger_id: String,
    pub user: SlackUser,
    pub view: SlackView,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackUser {
    pub id: String,
    pub username: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackChannel {
    pub id: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackView {
    pub id: String,
    pub callback_id: String,
    pub state: Option<SlackViewState>,
    pub private_metadata: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SlackViewState {
    pub values: HashMap<String, HashMap<String, serde_json::Value>>,
}

impl SlackRequest {
    pub fn parse(content_type: &str, body: &str) -> Result<Self, crate::Error> {
        if content_type.contains("application/x-www-form-urlencoded") {
            Self::parse_form(body)
        } else {
            Self::parse_json(body)
        }
    }

    fn parse_form(body: &str) -> Result<Self, crate::Error> {
        let params: HashMap<String, String> =
            serde_urlencoded::from_str(body).map_err(|e| crate::Error::Parse(e.to_string()))?;

        if let Some(payload) = params.get("payload") {
            return Self::parse_interaction_payload(payload);
        }

        if let Some(command) = params.get("command") {
            return Ok(SlackRequest::Command(SlackCommand {
                command: command.clone(),
                text: params.get("text").cloned().unwrap_or_default(),
                trigger_id: params.get("trigger_id").cloned().unwrap_or_default(),
                user_id: params.get("user_id").cloned().unwrap_or_default(),
                user_name: params.get("user_name").cloned().unwrap_or_default(),
                channel_id: params.get("channel_id").cloned().unwrap_or_default(),
                channel_name: params.get("channel_name").cloned().unwrap_or_default(),
                team_id: params.get("team_id").cloned().unwrap_or_default(),
                response_url: params.get("response_url").cloned().unwrap_or_default(),
            }));
        }

        Err(crate::Error::Parse(
            "unrecognized form payload".to_string(),
        ))
    }

    fn parse_interaction_payload(payload: &str) -> Result<Self, crate::Error> {
        let v: serde_json::Value = serde_json::from_str(payload)?;
        let payload_type = v["type"].as_str().unwrap_or("");

        match payload_type {
            "block_actions" => {
                let action_id = v["actions"][0]["action_id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                Ok(SlackRequest::Action(SlackAction {
                    action_id,
                    trigger_id: v["trigger_id"].as_str().unwrap_or("").to_string(),
                    user: serde_json::from_value(v["user"].clone())
                        .unwrap_or(SlackUser { id: String::new(), username: None, name: None }),
                    channel: serde_json::from_value(v["channel"].clone()).ok(),
                    response_url: v["response_url"].as_str().map(String::from),
                    actions: v["actions"].as_array().cloned().unwrap_or_default(),
                }))
            }
            "view_submission" => {
                let callback_id = v["view"]["callback_id"]
                    .as_str()
                    .unwrap_or("")
                    .to_string();
                Ok(SlackRequest::ViewSubmission(SlackViewSubmission {
                    callback_id,
                    trigger_id: v["trigger_id"].as_str().unwrap_or("").to_string(),
                    user: serde_json::from_value(v["user"].clone())
                        .unwrap_or(SlackUser { id: String::new(), username: None, name: None }),
                    view: serde_json::from_value(v["view"].clone())
                        .map_err(|e| crate::Error::Parse(e.to_string()))?,
                }))
            }
            other => Err(crate::Error::Parse(format!(
                "unknown interaction type: {other}"
            ))),
        }
    }

    fn parse_json(body: &str) -> Result<Self, crate::Error> {
        let v: serde_json::Value = serde_json::from_str(body)?;

        if let Some(challenge) = v.get("challenge") {
            return Ok(SlackRequest::UrlVerification {
                challenge: challenge.as_str().unwrap_or("").to_string(),
            });
        }

        if let Some(event) = v.get("event") {
            let event_type = event["type"].as_str().unwrap_or("").to_string();
            return Ok(SlackRequest::Event(SlackEvent {
                event_type,
                team_id: v["team_id"].as_str().unwrap_or("").to_string(),
                event: event.clone(),
                event_id: v["event_id"].as_str().unwrap_or("").to_string(),
            }));
        }

        Err(crate::Error::Parse(
            "unrecognized JSON payload".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_slash_command() {
        let body = "command=%2Fhello&text=world&user_id=U123&channel_id=C456&team_id=T789&trigger_id=tr1&user_name=testuser&channel_name=general&response_url=https%3A%2F%2Fhooks.slack.com%2Fcommands%2Fxyz";
        let req = SlackRequest::parse("application/x-www-form-urlencoded", body).unwrap();
        assert!(matches!(req, SlackRequest::Command(_)));
    }

    #[test]
    fn test_parse_command_fields() {
        let body = "command=%2Ftikical&text=upcoming&user_id=U111&channel_id=C222&team_id=T333&trigger_id=tr99&user_name=raj&channel_name=dev&response_url=https%3A%2F%2Fhooks.example.com%2Frespond";
        let req = SlackRequest::parse("application/x-www-form-urlencoded", body).unwrap();
        if let SlackRequest::Command(cmd) = req {
            assert_eq!(cmd.command, "/tikical");
            assert_eq!(cmd.text, "upcoming");
            assert_eq!(cmd.user_id, "U111");
            assert_eq!(cmd.channel_id, "C222");
            assert_eq!(cmd.team_id, "T333");
            assert_eq!(cmd.user_name, "raj");
            assert_eq!(cmd.channel_name, "dev");
            assert_eq!(cmd.trigger_id, "tr99");
            assert!(cmd.response_url.contains("hooks.example.com"));
        } else {
            panic!("expected Command variant");
        }
    }

    #[test]
    fn test_parse_url_verification() {
        let body = r#"{"type":"url_verification","challenge":"abc123","token":"xyz"}"#;
        let req = SlackRequest::parse("application/json", body).unwrap();
        if let SlackRequest::UrlVerification { challenge } = req {
            assert_eq!(challenge, "abc123");
        } else {
            panic!("expected UrlVerification variant");
        }
    }

    #[test]
    fn test_parse_event() {
        let body = r#"{"team_id":"T1","event_id":"Ev1","event":{"type":"app_mention","text":"hello","user":"U1"}}"#;
        let req = SlackRequest::parse("application/json", body).unwrap();
        if let SlackRequest::Event(evt) = req {
            assert_eq!(evt.event_type, "app_mention");
            assert_eq!(evt.team_id, "T1");
            assert_eq!(evt.event_id, "Ev1");
            assert_eq!(evt.event["text"], "hello");
        } else {
            panic!("expected Event variant");
        }
    }

    #[test]
    fn test_parse_block_action() {
        let payload = r#"{"type":"block_actions","trigger_id":"tr1","user":{"id":"U1"},"actions":[{"action_id":"btn_click","type":"button"}],"response_url":"https://hooks.example.com"}"#;
        let body = format!("payload={}", urlencoded(payload));
        let req = SlackRequest::parse("application/x-www-form-urlencoded", &body).unwrap();
        if let SlackRequest::Action(act) = req {
            assert_eq!(act.action_id, "btn_click");
            assert_eq!(act.trigger_id, "tr1");
            assert_eq!(act.user.id, "U1");
            assert_eq!(act.response_url.as_deref(), Some("https://hooks.example.com"));
        } else {
            panic!("expected Action variant");
        }
    }

    #[test]
    fn test_parse_view_submission() {
        let payload = r#"{"type":"view_submission","trigger_id":"tr2","user":{"id":"U2"},"view":{"id":"V1","callback_id":"create_event","state":{"values":{}},"private_metadata":"C123"}}"#;
        let body = format!("payload={}", urlencoded(payload));
        let req = SlackRequest::parse("application/x-www-form-urlencoded", &body).unwrap();
        if let SlackRequest::ViewSubmission(vs) = req {
            assert_eq!(vs.callback_id, "create_event");
            assert_eq!(vs.user.id, "U2");
            assert_eq!(vs.view.private_metadata.as_deref(), Some("C123"));
        } else {
            panic!("expected ViewSubmission variant");
        }
    }

    #[test]
    fn test_parse_unknown_interaction() {
        let payload = r#"{"type":"unknown_thing","trigger_id":"tr1","user":{"id":"U1"}}"#;
        let body = format!("payload={}", urlencoded(payload));
        let result = SlackRequest::parse("application/x-www-form-urlencoded", &body);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_form() {
        let result = SlackRequest::parse("application/x-www-form-urlencoded", "foo=bar");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_json() {
        let result = SlackRequest::parse("application/json", "not json at all{{{");
        assert!(result.is_err());
    }

    fn urlencoded(s: &str) -> String {
        serde_urlencoded::to_string(&[("", s)]).unwrap().strip_prefix('=').unwrap().to_string()
    }
}
