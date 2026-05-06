use lambda_http::{Body, Request, Response, run, service_fn};
use slack_volt::App;
use slack_volt::middleware::Headers;
use std::sync::Arc;

pub struct LambdaAdapter;

impl LambdaAdapter {
    pub async fn run(app: App) -> Result<(), lambda_http::Error> {
        let app = Arc::new(app);
        run(service_fn(move |req: Request| {
            let app = app.clone();
            async move { handle_request(req, &app).await }
        }))
        .await
    }
}

async fn handle_request(
    req: Request,
    app: &App,
) -> Result<Response<Body>, lambda_http::Error> {
    let headers = extract_headers(&req);
    let content_type = headers.content_type.clone();
    let body = match req.body() {
        Body::Text(s) => s.clone(),
        Body::Binary(b) => String::from_utf8_lossy(b).to_string(),
        Body::Empty => String::new(),
    };

    tracing::debug!(content_type = %content_type, body_len = body.len(), "incoming slack request");

    match app.dispatch_async(&content_type, &body, headers).await {
        Ok(ack) => {
            if ack.is_empty() {
                let resp = Response::builder()
                    .status(200)
                    .body(Body::Empty)?;
                Ok(resp)
            } else {
                let response_body = serde_json::to_string(&ack)?;
                let resp = Response::builder()
                    .status(200)
                    .header("content-type", "application/json")
                    .body(Body::Text(response_body))?;
                Ok(resp)
            }
        }
        Err(slack_volt::Error::NoHandler { kind, id }) => {
            tracing::warn!(kind, id, "no handler registered");
            let resp = Response::builder()
                .status(200)
                .body(Body::Empty)?;
            Ok(resp)
        }
        Err(slack_volt::Error::SignatureVerification(msg)) => {
            tracing::error!(msg, "signature verification failed");
            let resp = Response::builder()
                .status(401)
                .body(Body::Text("unauthorized".to_string()))?;
            Ok(resp)
        }
        Err(e) => {
            tracing::error!(error = %e, "handler error");
            let resp = Response::builder()
                .status(200)
                .body(Body::Empty)?;
            Ok(resp)
        }
    }
}

fn extract_headers(req: &Request) -> Headers {
    let get = |name: &str| {
        req.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string()
    };

    Headers {
        timestamp: get("x-slack-request-timestamp"),
        signature: get("x-slack-signature"),
        content_type: get("content-type"),
    }
}
