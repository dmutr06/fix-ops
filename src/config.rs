//! Runtime configuration read from environment variables.

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, bail};
use fixops_domain::staff::{Password, Username};

/// Service settings. The variables and their defaults are listed in the README.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub listen_addr: SocketAddr,
    pub db_max_connections: u32,
    /// Directory of the daily operational log files.
    pub log_dir: PathBuf,
    pub bootstrap_admin: Option<BootstrapAdmin>,
}

/// The first Administrator, created at startup while no staff account exists.
#[derive(Debug, Clone)]
pub struct BootstrapAdmin {
    pub username: Username,
    pub password: Password,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            database_url: std::env::var("DATABASE_URL").context("DATABASE_URL is not set")?,
            listen_addr: var_or("FIXOPS_LISTEN_ADDR", "127.0.0.1:8080")?,
            db_max_connections: var_or("FIXOPS_DB_MAX_CONNECTIONS", "10")?,
            log_dir: var_or("FIXOPS_LOG_DIR", "var/log")?,
            bootstrap_admin: bootstrap_admin()?,
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

fn bootstrap_admin() -> anyhow::Result<Option<BootstrapAdmin>> {
    const USERNAME: &str = "FIXOPS_BOOTSTRAP_ADMIN_USERNAME";
    const PASSWORD: &str = "FIXOPS_BOOTSTRAP_ADMIN_PASSWORD";

    match (std::env::var(USERNAME).ok(), std::env::var(PASSWORD).ok()) {
        (None, None) => Ok(None),
        (Some(username), Some(password)) => Ok(Some(BootstrapAdmin {
            username: Username::parse(&username)
                .with_context(|| format!("{USERNAME} is invalid"))?,
            password: Password::parse(password)
                .with_context(|| format!("{PASSWORD} is invalid"))?,
        })),
        _ => bail!("set both {USERNAME} and {PASSWORD}, or neither"),
    }
}
