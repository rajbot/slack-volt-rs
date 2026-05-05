use std::collections::HashMap;
use std::sync::Arc;

use crate::context::{
    ActionContext, CommandContext, EventContext, SlackClient, ViewSubmissionContext,
};
use crate::handler::Handler;
use crate::installation::InstallationStore;
use crate::middleware::{Headers, Middleware, SignatureVerifier};
use crate::request::SlackRequest;
use crate::response::AckResponse;
use crate::Error;

type BoxedCommandHandler = Box<dyn Handler<CommandContext>>;
type BoxedEventHandler = Box<dyn Handler<EventContext>>;
type BoxedActionHandler = Box<dyn Handler<ActionContext>>;
type BoxedViewHandler = Box<dyn Handler<ViewSubmissionContext>>;

pub struct App {
    pub(crate) commands: HashMap<String, Arc<BoxedCommandHandler>>,
    pub(crate) events: HashMap<String, Arc<BoxedEventHandler>>,
    pub(crate) actions: HashMap<String, Arc<BoxedActionHandler>>,
    pub(crate) view_submissions: HashMap<String, Arc<BoxedViewHandler>>,
    pub(crate) middleware: Vec<Box<dyn Middleware>>,
    pub(crate) bot_token: String,
    pub(crate) slack_api_base_url: String,
    pub(crate) http: reqwest::Client,
    pub(crate) installation_store: Option<Arc<dyn InstallationStore>>,
}

impl App {
    pub fn new() -> AppBuilder {
        AppBuilder::default()
    }

    fn make_client(&self) -> SlackClient {
        SlackClient::with_http(self.http.clone(), self.bot_token.clone(), self.slack_api_base_url.clone())
    }

    async fn resolve_client(&self, team_id: &str) -> Result<SlackClient, Error> {
        if let Some(ref store) = self.installation_store {
            let token = store.fetch_bot_token(team_id).await?;
            Ok(SlackClient::with_http(self.http.clone(), token, self.slack_api_base_url.clone()))
        } else {
            Ok(self.make_client())
        }
    }

    pub fn dispatch(
        &self,
        content_type: &str,
        body: &str,
        headers: Headers,
    ) -> Result<AckResponse, Error> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current()
                .block_on(self.dispatch_async(content_type, body, headers))
        })
    }

    pub async fn dispatch_async(
        &self,
        content_type: &str,
        body: &str,
        headers: Headers,
    ) -> Result<AckResponse, Error> {
        for mw in &self.middleware {
            mw.process(&headers, body)?;
        }

        let request = SlackRequest::parse(content_type, body)?;
        match request {
            SlackRequest::UrlVerification { challenge } => {
                Ok(AckResponse::text(challenge))
            }
            SlackRequest::Command(cmd) => {
                let handler = self
                    .commands
                    .get(&cmd.command)
                    .ok_or_else(|| Error::NoHandler {
                        kind: "command",
                        id: cmd.command.clone(),
                    })?
                    .clone();
                let client = self.resolve_client(&cmd.team_id).await?;
                let ctx = CommandContext::new(cmd, client);
                handler.call(ctx).await
            }
            SlackRequest::Event(evt) => {
                let handler = self
                    .events
                    .get(&evt.event_type)
                    .ok_or_else(|| Error::NoHandler {
                        kind: "event",
                        id: evt.event_type.clone(),
                    })?
                    .clone();
                let client = self.resolve_client(&evt.team_id).await?;
                let ctx = EventContext::new(evt, client);
                handler.call(ctx).await
            }
            SlackRequest::Action(act) => {
                let handler = self
                    .actions
                    .get(&act.action_id)
                    .ok_or_else(|| Error::NoHandler {
                        kind: "action",
                        id: act.action_id.clone(),
                    })?
                    .clone();
                let client = self.resolve_client(&act.team_id).await?;
                let ctx = ActionContext::new(act, client);
                handler.call(ctx).await
            }
            SlackRequest::ViewSubmission(vs) => {
                let handler = self
                    .view_submissions
                    .get(&vs.callback_id)
                    .ok_or_else(|| Error::NoHandler {
                        kind: "view_submission",
                        id: vs.callback_id.clone(),
                    })?
                    .clone();
                let client = self.resolve_client(&vs.team_id).await?;
                let ctx = ViewSubmissionContext::new(vs, client);
                handler.call(ctx).await
            }
        }
    }
}

#[derive(Default)]
pub struct AppBuilder {
    commands: HashMap<String, Arc<BoxedCommandHandler>>,
    events: HashMap<String, Arc<BoxedEventHandler>>,
    actions: HashMap<String, Arc<BoxedActionHandler>>,
    view_submissions: HashMap<String, Arc<BoxedViewHandler>>,
    signing_secret: Option<String>,
    bot_token: Option<String>,
    slack_api_base_url: Option<String>,
    installation_store: Option<Arc<dyn InstallationStore>>,
}

impl AppBuilder {
    pub fn command(mut self, name: &str, handler: impl Handler<CommandContext>) -> Self {
        self.commands
            .insert(name.to_string(), Arc::new(Box::new(handler)));
        self
    }

    pub fn event(mut self, event_type: &str, handler: impl Handler<EventContext>) -> Self {
        self.events
            .insert(event_type.to_string(), Arc::new(Box::new(handler)));
        self
    }

    pub fn action(mut self, action_id: &str, handler: impl Handler<ActionContext>) -> Self {
        self.actions
            .insert(action_id.to_string(), Arc::new(Box::new(handler)));
        self
    }

    pub fn view_submission(
        mut self,
        callback_id: &str,
        handler: impl Handler<ViewSubmissionContext>,
    ) -> Self {
        self.view_submissions
            .insert(callback_id.to_string(), Arc::new(Box::new(handler)));
        self
    }

    pub fn signing_secret(mut self, secret: impl Into<String>) -> Self {
        self.signing_secret = Some(secret.into());
        self
    }

    pub fn bot_token(mut self, token: impl Into<String>) -> Self {
        self.bot_token = Some(token.into());
        self
    }

    pub fn slack_api_base_url(mut self, url: impl Into<String>) -> Self {
        self.slack_api_base_url = Some(url.into());
        self
    }

    pub fn installation_store(mut self, store: Arc<dyn InstallationStore>) -> Self {
        self.installation_store = Some(store);
        self
    }

    pub fn build(self) -> App {
        let mut middleware: Vec<Box<dyn Middleware>> = Vec::new();

        if let Some(secret) = self.signing_secret {
            middleware.push(Box::new(SignatureVerifier::new(secret)));
        }

        App {
            commands: self.commands,
            events: self.events,
            actions: self.actions,
            view_submissions: self.view_submissions,
            middleware,
            bot_token: self.bot_token.unwrap_or_default(),
            slack_api_base_url: self.slack_api_base_url.unwrap_or_else(|| "https://slack.com/api".to_string()),
            http: reqwest::Client::new(),
            installation_store: self.installation_store,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::CommandContext;
    use crate::response::AckResponse;

    async fn echo_handler(mut ctx: CommandContext) -> Result<AckResponse, crate::Error> {
        Ok(ctx.ack(format!("echo: {}", ctx.command.text)))
    }

    fn no_sig_headers() -> Headers {
        Headers {
            timestamp: String::new(),
            signature: String::new(),
            content_type: "application/x-www-form-urlencoded".to_string(),
        }
    }

    #[tokio::test]
    async fn test_app_dispatches_command() {
        let app = App::new()
            .command("/test", echo_handler)
            .build();

        let body = "command=%2Ftest&text=hello&user_id=U1&channel_id=C1&team_id=T1&trigger_id=tr1&user_name=u&channel_name=c&response_url=http%3A%2F%2Fexample.com";
        let result = app.dispatch_async(
            "application/x-www-form-urlencoded",
            body,
            no_sig_headers(),
        ).await.unwrap();

        assert_eq!(result.text.as_deref(), Some("echo: hello"));
    }

    #[tokio::test]
    async fn test_app_dispatches_url_verification() {
        let app = App::new().build();
        let body = r#"{"type":"url_verification","challenge":"test_challenge"}"#;
        let headers = Headers {
            timestamp: String::new(),
            signature: String::new(),
            content_type: "application/json".to_string(),
        };
        let result = app.dispatch_async("application/json", body, headers).await.unwrap();
        assert_eq!(result.text.as_deref(), Some("test_challenge"));
    }

    #[tokio::test]
    async fn test_app_no_handler_error() {
        let app = App::new().build();
        let body = "command=%2Funknown&text=x&user_id=U1&channel_id=C1&team_id=T1&trigger_id=tr1&user_name=u&channel_name=c&response_url=http%3A%2F%2Fexample.com";
        let result = app.dispatch_async(
            "application/x-www-form-urlencoded",
            body,
            no_sig_headers(),
        ).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), crate::Error::NoHandler { .. }));
    }

    #[tokio::test]
    async fn test_app_multiple_commands() {
        async fn handler_a(mut ctx: CommandContext) -> Result<AckResponse, crate::Error> {
            Ok(ctx.ack("from A"))
        }
        async fn handler_b(mut ctx: CommandContext) -> Result<AckResponse, crate::Error> {
            Ok(ctx.ack("from B"))
        }

        let app = App::new()
            .command("/alpha", handler_a)
            .command("/beta", handler_b)
            .build();

        let body_a = "command=%2Falpha&text=&user_id=U1&channel_id=C1&team_id=T1&trigger_id=tr1&user_name=u&channel_name=c&response_url=http%3A%2F%2Fex.com";
        let body_b = "command=%2Fbeta&text=&user_id=U1&channel_id=C1&team_id=T1&trigger_id=tr1&user_name=u&channel_name=c&response_url=http%3A%2F%2Fex.com";

        let result_a = app.dispatch_async("application/x-www-form-urlencoded", body_a, no_sig_headers()).await.unwrap();
        let result_b = app.dispatch_async("application/x-www-form-urlencoded", body_b, no_sig_headers()).await.unwrap();

        assert_eq!(result_a.text.as_deref(), Some("from A"));
        assert_eq!(result_b.text.as_deref(), Some("from B"));
    }

    #[tokio::test]
    async fn test_app_dispatches_event() {
        async fn mention_handler(_ctx: crate::EventContext) -> Result<AckResponse, crate::Error> {
            Ok(AckResponse::empty())
        }

        let app = App::new()
            .event("app_mention", mention_handler)
            .build();

        let body = r#"{"team_id":"T1","event_id":"Ev1","event":{"type":"app_mention","text":"hi"}}"#;
        let headers = Headers {
            timestamp: String::new(),
            signature: String::new(),
            content_type: "application/json".to_string(),
        };
        let result = app.dispatch_async("application/json", body, headers).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_app_dispatches_action() {
        async fn action_handler(mut ctx: crate::ActionContext) -> Result<AckResponse, crate::Error> {
            Ok(ctx.ack())
        }

        let app = App::new()
            .action("btn_click", action_handler)
            .build();

        let payload = r#"{"type":"block_actions","trigger_id":"tr1","user":{"id":"U1"},"actions":[{"action_id":"btn_click","type":"button"}]}"#;
        let encoded = serde_urlencoded::to_string(&[("payload", payload)]).unwrap();
        let result = app.dispatch_async("application/x-www-form-urlencoded", &encoded, no_sig_headers()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_app_dispatches_view_submission() {
        async fn view_handler(mut ctx: crate::ViewSubmissionContext) -> Result<AckResponse, crate::Error> {
            Ok(ctx.ack())
        }

        let app = App::new()
            .view_submission("my_form", view_handler)
            .build();

        let payload = r#"{"type":"view_submission","trigger_id":"tr1","user":{"id":"U1"},"view":{"id":"V1","callback_id":"my_form","state":{"values":{}},"private_metadata":""}}"#;
        let encoded = serde_urlencoded::to_string(&[("payload", payload)]).unwrap();
        let result = app.dispatch_async("application/x-www-form-urlencoded", &encoded, no_sig_headers()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_app_with_signature_verification() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        let secret = "test_secret";
        let app = App::new()
            .signing_secret(secret)
            .command("/hello", echo_handler)
            .build();

        let body = "command=%2Fhello&text=signed&user_id=U1&channel_id=C1&team_id=T1&trigger_id=tr1&user_name=u&channel_name=c&response_url=http%3A%2F%2Fex.com";
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let basestring = format!("v0:{now}:{body}");
        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(basestring.as_bytes());
        let sig = format!("v0={}", hex::encode(mac.finalize().into_bytes()));

        let headers = Headers {
            timestamp: now,
            signature: sig,
            content_type: "application/x-www-form-urlencoded".to_string(),
        };
        let result = app.dispatch_async("application/x-www-form-urlencoded", body, headers).await.unwrap();
        assert_eq!(result.text.as_deref(), Some("echo: signed"));
    }

    #[tokio::test]
    async fn test_app_rejects_bad_signature() {
        let app = App::new()
            .signing_secret("real_secret")
            .command("/hello", echo_handler)
            .build();

        let body = "command=%2Fhello&text=x&user_id=U1&channel_id=C1&team_id=T1&trigger_id=tr1&user_name=u&channel_name=c&response_url=http%3A%2F%2Fex.com";
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            .to_string();

        let headers = Headers {
            timestamp: now,
            signature: "v0=badbadbadbad".to_string(),
            content_type: "application/x-www-form-urlencoded".to_string(),
        };
        let result = app.dispatch_async("application/x-www-form-urlencoded", body, headers).await;
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), crate::Error::SignatureVerification(_)));
    }

    #[tokio::test]
    async fn test_app_parse_error_on_garbage_body() {
        let app = App::new().build();
        let headers = Headers {
            timestamp: String::new(),
            signature: String::new(),
            content_type: "application/json".to_string(),
        };
        let result = app.dispatch_async("application/json", "{{invalid", headers).await;
        assert!(result.is_err());
    }
}
