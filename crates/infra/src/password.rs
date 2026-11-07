//! [`PasswordHasher`] with argon2id. Hashing is CPU-heavy, so it runs on the blocking pool
//! instead of an async worker thread.

use std::sync::LazyLock;

use argon2::{Argon2, password_hash};
use fixops_app::StoreError;
use fixops_app::staff::{PasswordHash, PasswordHasher};
use fixops_domain::staff::Password;
use tokio::task::spawn_blocking;

use crate::PgStore;

/// A hash with the same argon2 parameters as real accounts, checked when a username does not
/// exist so that sign-in takes as long as for a wrong password.
static DUMMY_HASH: LazyLock<Result<String, password_hash::Error>> =
    LazyLock::new(|| hash("unknown-account-dummy-password"));

fn hash(password: &str) -> Result<String, password_hash::Error> {
    argon2::PasswordHasher::hash_password(&Argon2::default(), password.as_bytes())
        .map(|hash| hash.to_string())
}

/// Checks `password` against `hash`, or against [`DUMMY_HASH`] when there is no account.
/// Without an account the answer is always `false`.
fn verify(password: &str, hash: Option<&str>) -> Result<bool, password_hash::Error> {
    let (target, exists) = match hash {
        Some(hash) => (hash, true),
        None => (DUMMY_HASH.as_deref().map_err(Clone::clone)?, false),
    };
    match argon2::PasswordVerifier::verify_password(&Argon2::default(), password.as_bytes(), target)
    {
        Ok(()) => Ok(exists),
        Err(password_hash::Error::PasswordInvalid) => Ok(false),
        Err(err) => Err(err),
    }
}

fn hasher_error(err: impl std::error::Error + Send + Sync + 'static) -> StoreError {
    StoreError::Other(Box::new(err))
}

impl PasswordHasher for PgStore {
    async fn hash_password(&self, password: &Password) -> Result<PasswordHash, StoreError> {
        let password = password.expose().to_owned();
        let hash = spawn_blocking(move || hash(&password))
            .await
            .map_err(hasher_error)?
            .map_err(hasher_error)?;
        Ok(PasswordHash(hash))
    }

    async fn verify_password(
        &self,
        password: &str,
        hash: Option<&PasswordHash>,
    ) -> Result<bool, StoreError> {
        let password = password.to_owned();
        let hash = hash.map(|hash| hash.0.clone());
        spawn_blocking(move || verify(&password, hash.as_deref()))
            .await
            .map_err(hasher_error)?
            .map_err(hasher_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verifies_real_hashes() {
        let stored = hash("correct-horse-battery").unwrap();
        assert!(stored.starts_with("$argon2id$"));
        assert_eq!(verify("correct-horse-battery", Some(&stored)), Ok(true));
        assert_eq!(verify("wrong-password", Some(&stored)), Ok(false));
    }

    #[test]
    fn missing_account_never_verifies() {
        assert_eq!(verify("unknown-account-dummy-password", None), Ok(false));
        assert_eq!(verify("anything", None), Ok(false));
    }

    #[test]
    fn dummy_hash_uses_the_account_parameters() {
        let stored = hash("correct-horse-battery").unwrap();
        let dummy = DUMMY_HASH.as_ref().unwrap();
        let params = |h: &str| h.rsplitn(3, '$').nth(2).map(str::to_owned);
        assert_eq!(params(dummy), params(&stored));
    }
}
