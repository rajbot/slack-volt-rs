use crate::Error;

#[async_trait::async_trait]
pub trait InstallationStore: Send + Sync {
    async fn fetch_bot_token(&self, team_id: &str) -> Result<String, Error>;
}
