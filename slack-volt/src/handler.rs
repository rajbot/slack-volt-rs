use crate::context::{ActionContext, CommandContext, EventContext, ViewSubmissionContext};
use crate::response::AckResponse;
use std::future::Future;
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Handler<Ctx>: Send + Sync + 'static {
    fn call(&self, ctx: Ctx) -> BoxFuture<'_, Result<AckResponse, crate::Error>>;
}

impl<F, Fut> Handler<CommandContext> for F
where
    F: Fn(CommandContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<AckResponse, crate::Error>> + Send + 'static,
{
    fn call(&self, ctx: CommandContext) -> BoxFuture<'_, Result<AckResponse, crate::Error>> {
        Box::pin(self(ctx))
    }
}

impl<F, Fut> Handler<EventContext> for F
where
    F: Fn(EventContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<AckResponse, crate::Error>> + Send + 'static,
{
    fn call(&self, ctx: EventContext) -> BoxFuture<'_, Result<AckResponse, crate::Error>> {
        Box::pin(self(ctx))
    }
}

impl<F, Fut> Handler<ActionContext> for F
where
    F: Fn(ActionContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<AckResponse, crate::Error>> + Send + 'static,
{
    fn call(&self, ctx: ActionContext) -> BoxFuture<'_, Result<AckResponse, crate::Error>> {
        Box::pin(self(ctx))
    }
}

impl<F, Fut> Handler<ViewSubmissionContext> for F
where
    F: Fn(ViewSubmissionContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<AckResponse, crate::Error>> + Send + 'static,
{
    fn call(
        &self,
        ctx: ViewSubmissionContext,
    ) -> BoxFuture<'_, Result<AckResponse, crate::Error>> {
        Box::pin(self(ctx))
    }
}
