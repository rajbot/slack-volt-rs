use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AckResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocks: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_type: Option<String>,
}

impl AckResponse {
    pub fn empty() -> Self {
        AckResponse {
            text: None,
            blocks: None,
            response_type: None,
        }
    }

    pub fn text(msg: impl Into<String>) -> Self {
        AckResponse {
            text: Some(msg.into()),
            blocks: None,
            response_type: None,
        }
    }

    pub fn ephemeral(msg: impl Into<String>) -> Self {
        AckResponse {
            text: Some(msg.into()),
            blocks: None,
            response_type: Some("ephemeral".to_string()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_none() && self.blocks.is_none() && self.response_type.is_none()
    }

    pub fn blocks(blocks: Vec<serde_json::Value>) -> Self {
        AckResponse {
            text: None,
            blocks: Some(blocks),
            response_type: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ack_text_serializes() {
        let ack = AckResponse::text("hi");
        let json: serde_json::Value = serde_json::to_value(&ack).unwrap();
        assert_eq!(json["text"], "hi");
        assert!(json.get("blocks").is_none());
        assert!(json.get("response_type").is_none());
    }

    #[test]
    fn test_ack_empty_serializes() {
        let ack = AckResponse::empty();
        let json: serde_json::Value = serde_json::to_value(&ack).unwrap();
        assert!(json.get("text").is_none());
        assert!(json.get("blocks").is_none());
    }

    #[test]
    fn test_ack_ephemeral_serializes() {
        let ack = AckResponse::ephemeral("secret message");
        let json: serde_json::Value = serde_json::to_value(&ack).unwrap();
        assert_eq!(json["text"], "secret message");
        assert_eq!(json["response_type"], "ephemeral");
    }
}
