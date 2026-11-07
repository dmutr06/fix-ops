//! [`PasswordHasher`] with argon2id. Hashing is CPU-heavy, so it runs on the blocking pool
//! instead of an async worker thread.

use argon2::{Argon2, password_hash};
use fixops_app::StoreError;
use fixops_app::staff::{PasswordHash, PasswordHasher};
use fixops_domain::staff::Password;
use tokio::task::spawn_blocking;

use crate::PgStore;

fn hasher_error(err: impl std::error::Error + Send + Sync + 'static) -> StoreError {
    StoreError::Other(Box::new(err))
}

impl PasswordHasher for PgStore {
    async fn hash_password(&self, password: &Password) -> Result<PasswordHash, StoreError> {
        let password = password.expose().to_owned();
        let hash = spawn_blocking(move || {
            argon2::PasswordHasher::hash_password(&Argon2::default(), password.as_bytes())
                .map(|hash| hash.to_string())
        })
        .await
        .map_err(hasher_error)?
        .map_err(hasher_error)?;
        Ok(PasswordHash(hash))
    }

    async fn verify_password(
        &self,
        password: &str,
        hash: &PasswordHash,
    ) -> Result<bool, StoreError> {
        let password = password.to_owned();
        let hash = hash.0.clone();
        let verified = spawn_blocking(move || {
            argon2::PasswordVerifier::verify_password(
                &Argon2::default(),
                password.as_bytes(),
                hash.as_str(),
            )
        })
        .await
        .map_err(hasher_error)?;
        match verified {
            Ok(()) => Ok(true),
            Err(password_hash::Error::PasswordInvalid) => Ok(false),
            Err(err) => Err(hasher_error(err)),
        }
    }
}
