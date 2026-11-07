//! Operational logging: one line per request and per action outcome, to the console and to a
//! daily JSON file. This is not the audit log. The audit log records what changed in the
//! database; this log records everything the service did, for analysing a run afterwards.
//!
//! Each layer logs what it knows:
//! - HTTP: a `request` span (request id, method, path, actor, action) closed by a
//!   `request finished` line with status and latency, and a `request rejected` line per error.
//! - App: `action denied` when a role may not perform an action.
//! - Infra: `record changed` for every saved change, with field names but not values.

use std::path::Path;

use tracing::Subscriber;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

/// Default filter when `RUST_LOG` is not set.
pub const DEFAULT_FILTER: &str = "info";

/// File name prefix of the daily log files, e.g. `fixops.log.2026-11-07`.
pub const LOG_FILE_PREFIX: &str = "fixops.log";

/// A JSON-lines layer writing to `writer`, with the current span's fields on every line.
pub fn json_layer<S, W>(writer: W) -> impl Layer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    W: for<'w> MakeWriter<'w> + Send + Sync + 'static,
{
    tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_writer(writer)
}

/// Installs the global subscriber: readable console output plus a daily JSON file in `log_dir`.
///
/// Keep the returned guard alive until shutdown; dropping it flushes the file.
pub fn init(log_dir: &Path) -> WorkerGuard {
    let file = tracing_appender::rolling::daily(log_dir, LOG_FILE_PREFIX);
    let (file, guard) = tracing_appender::non_blocking(file);
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| DEFAULT_FILTER.into());

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(json_layer(file))
        .init();
    guard
}
