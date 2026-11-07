//! Runtime configuration read from environment variables.

use std::net::SocketAddr;

use anyhow::Context;

/// Service settings. The variables and their defaults are listed in the README.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub listen_addr: SocketAddr,
    pub db_max_connections: u32,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL is not set")?,
            listen_addr: var_or("FIXOPS_LISTEN_ADDR", "127.0.0.1:8080")?,
            db_max_connections: var_or("FIXOPS_DB_MAX_CONNECTIONS", "10")?,
        })
    }
}

fn var_or<T>(name: &str, default: &str) -> anyhow::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    std::env::var(name)
        .unwrap_or_else(|_| default.to_owned())
        .parse()
        .with_context(|| format!("{name} is invalid"))
}
