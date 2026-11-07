//! Request logging: a `request` span per request and one `request finished` line at the end.

use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderName, Request, Response};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::{Span, field};

const REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

/// Adds request ids (`x-request-id`, generated when missing and echoed back) and request logging.
pub fn layer<S>(router: Router<S>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    let trace = TraceLayer::new_for_http()
        .make_span_with(make_span)
        .on_request(())
        .on_response(on_response)
        .on_failure(());

    // Layers run outside-in: the id is set before the span is made and propagated last.
    router
        .layer(PropagateRequestIdLayer::new(REQUEST_ID))
        .layer(trace)
        .layer(SetRequestIdLayer::new(REQUEST_ID, MakeRequestUuid))
}

/// Only the path is logged, never the query string or headers, so no secret reaches the log.
fn make_span(request: &Request<Body>) -> Span {
    let request_id = request
        .headers()
        .get(&REQUEST_ID)
        .and_then(|id| id.to_str().ok())
        .unwrap_or_default();
    tracing::info_span!(
        "request",
        request_id,
        method = %request.method(),
        path = request.uri().path(),
        actor_id = field::Empty,
        role = field::Empty,
        action = field::Empty,
    )
}

fn on_response(response: &Response<Body>, latency: Duration, _span: &Span) {
    let status = response.status();
    let latency_ms = u64::try_from(latency.as_millis()).unwrap_or(u64::MAX);
    let status = status.as_u16();
    if response.status().is_server_error() {
        tracing::error!(status, latency_ms, "request finished");
    } else if response.status().is_client_error() {
        tracing::warn!(status, latency_ms, "request finished");
    } else {
        tracing::info!(status, latency_ms, "request finished");
    }
}

/// Records the signed-in actor on the current request span.
pub fn record_actor(actor: fixops_domain::staff::Actor) {
    let span = Span::current();
    span.record("actor_id", actor.id.0);
    span.record("role", actor.role.as_str());
}
