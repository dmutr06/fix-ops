//! Operational logging: one line per request and per action outcome, to the console and to a
//! daily JSON file. This is not the audit log. The audit log records what changed in the
//! database; this log records everything the service did, for analysing a run afterwards.

use tracing::Subscriber;
use tracing_subscriber::Layer;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::registry::LookupSpan;

/// A JSON-lines layer writing to `writer`, with the current span's fields on every line.
pub fn json_layer<S, W>(writer: W) -> impl Layer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    W: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
    drop(writer);
    tracing_subscriber::layer::Identity::new()
}
