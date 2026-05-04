use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("signature verification failed: {0}")]
    SignatureVerification(String),

    #[error("parse error: {0}")]
    Parse(String),

    #[error("no handler registered for {kind}: {id}")]
    NoHandler { kind: &'static str, id: String },

    #[error("slack API error: {0}")]
    SlackApi(String),

    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn other(msg: impl fmt::Display) -> Self {
        Error::Other(msg.to_string())
    }
}
