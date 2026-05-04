use std::collections::HashMap;
use std::sync::Arc;

use crate::context::{
    ActionContext, CommandContext, EventContext, SlackClient, ViewSubmissionContext,
};
use crate::handler::Handler;
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
}

impl App {
    pub fn new() -> AppBuilder {
        AppBuilder::default()
    }

    pub fn dispatch(
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
                let client = SlackClient::new(self.bot_token.clone());
                let ctx = CommandContext::new(cmd, client);
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(handler.call(ctx))
                })
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
                let client = SlackClient::new(self.bot_token.clone());
                let ctx = EventContext::new(evt, client);
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(handler.call(ctx))
                })
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
                let client = SlackClient::new(self.bot_token.clone());
                let ctx = ActionContext::new(act, client);
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(handler.call(ctx))
                })
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
                let client = SlackClient::new(self.bot_token.clone());
                let ctx = ViewSubmissionContext::new(vs, client);
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(handler.call(ctx))
                })
            }
        }
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
                let client = SlackClient::new(self.bot_token.clone());
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
                let client = SlackClient::new(self.bot_token.clone());
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
                let client = SlackClient::new(self.bot_token.clone());
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
                let client = SlackClient::new(self.bot_token.clone());
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
}
