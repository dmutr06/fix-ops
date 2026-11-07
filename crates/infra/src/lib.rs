//! Adapters for the ports declared in `fixops-app`: PostgreSQL storage via sqlx and argon2
//! password hashing.

mod audit;
mod clients;
mod devices;
mod error;
mod orders;
mod password;
mod staff;

use sqlx::PgPool;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;

/// Schema migrations, embedded at compile time from `crates/infra/migrations`.
pub static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

/// PostgreSQL implementation of every store port in `fixops-app`.
///
/// Each port is implemented in its own module. A write runs in one transaction together with
/// its audit entries.
#[derive(Debug, Clone)]
pub struct PgStore {
    pool: PgPool,
}

impl PgStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

/// Opens a connection pool to `database_url`.
pub async fn connect(database_url: &str, max_connections: u32) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
}

/// A pattern for `ILIKE` that matches `text` anywhere, with `%`, `_` and `\` in `text` taken
/// literally.
pub(crate) fn contains_pattern(text: &str) -> String {
    let mut pattern = String::with_capacity(text.len() + 2);
    pattern.push('%');
    for c in text.chars() {
        if matches!(c, '%' | '_' | '\\') {
            pattern.push('\\');
        }
        pattern.push(c);
    }
    pattern.push('%');
    pattern
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contains_pattern_escapes_wildcards() {
        assert_eq!(contains_pattern("vance"), "%vance%");
        assert_eq!(contains_pattern("50%_off\\"), "%50\\%\\_off\\\\%");
    }
}
