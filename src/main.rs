use anyhow::Context;
use fixops::config::Config;
use fixops::{AppState, router};
use fixops_infra::{MIGRATOR, PgStore};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    let _log_guard = fixops::telemetry::init(&config.log_dir);
    let pool = fixops_infra::connect(&config.database_url, config.db_max_connections)
        .await
        .context("cannot connect to the database")?;
    MIGRATOR.run(&pool).await.context("migrations failed")?;

    let state = AppState::new(PgStore::new(pool));
    if let Some(admin) = config.bootstrap_admin {
        let created = state
            .app
            .bootstrap_admin(admin.username, admin.password)
            .await
            .context("cannot create the first administrator")?;
        if let Some(account) = created {
            tracing::info!(username = %account.username, "created the first administrator");
        } else {
            tracing::info!("staff accounts exist, skipping administrator bootstrap");
        }
    }

    let app = router(state);
    let listener = tokio::net::TcpListener::bind(config.listen_addr).await?;
    tracing::info!(addr = %config.listen_addr, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutting down");
}
