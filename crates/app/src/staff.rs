//! Staff sign-in, sessions and account management (REQ-F-001, REQ-F-002, REQ-F-019).

use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use fixops_domain::audit::FieldChange;
use fixops_domain::id::StaffId;
use fixops_domain::staff::{
    Action, Actor, NewStaffAccount, Password, Role, StaffAccount, Username,
};
use sha2::{Digest, Sha256};
use time::{Duration, OffsetDateTime};

use crate::{App, AppError, AppResult, ErrorKind, StoreError};

/// How long a session token stays valid after sign-in.
pub const SESSION_LIFETIME: Duration = Duration::hours(12);

/// A stored password hash (PHC string). Its `Debug` output never shows the hash.
#[derive(Clone, PartialEq, Eq)]
pub struct PasswordHash(pub String);

impl fmt::Debug for PasswordHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PasswordHash(***)")
    }
}

/// An account together with its password hash, for checking a sign-in.
#[derive(Debug, Clone)]
pub struct Credentials {
    pub account: StaffAccount,
    pub password_hash: PasswordHash,
}

/// A stored session: the account it belongs to and when it expires.
#[derive(Debug, Clone)]
pub struct Session {
    pub account: StaffAccount,
    pub expires_at: OffsetDateTime,
}

/// A bearer token: 32 random bytes, base64url-encoded. It is shown to the client once and
/// stored only as its [`TokenHash`].
#[derive(Clone, PartialEq, Eq)]
pub struct SessionToken(String);

impl SessionToken {
    fn generate() -> Self {
        let bytes: [u8; 32] = rand::random();
        Self(URL_SAFE_NO_PAD.encode(bytes))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The token text, to hand to the client.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Debug for SessionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SessionToken(***)")
    }
}

/// SHA-256 of a session token, the only form in which a token is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TokenHash(pub [u8; 32]);

impl TokenHash {
    pub fn of(token: &str) -> Self {
        Self(Sha256::digest(token.as_bytes()).into())
    }
}

/// A successful sign-in.
#[derive(Debug, Clone)]
pub struct NewSession {
    pub token: SessionToken,
    pub account: StaffAccount,
}

/// Port: storage of staff accounts and sessions.
pub trait StaffStore: Send + Sync {
    /// All accounts, ordered by id.
    fn list_staff(&self) -> impl Future<Output = Result<Vec<StaffAccount>, StoreError>> + Send;

    fn find_staff(
        &self,
        id: StaffId,
    ) -> impl Future<Output = Result<Option<StaffAccount>, StoreError>> + Send;

    fn find_credentials(
        &self,
        username: &Username,
    ) -> impl Future<Output = Result<Option<Credentials>, StoreError>> + Send;

    /// Inserts an enabled account and its audit `changes`, attributed to `actor`, in one
    /// transaction. A taken username fails with [`StoreError::Duplicate`].
    fn create_staff(
        &self,
        actor: StaffId,
        account: &NewStaffAccount,
        password_hash: &PasswordHash,
        changes: &[FieldChange],
    ) -> impl Future<Output = Result<StaffAccount, StoreError>> + Send;

    /// Like [`StaffStore::create_staff`], but only while no account exists, and attributed to
    /// the new account itself. Returns `None` when an account already exists.
    fn create_first_staff(
        &self,
        account: &NewStaffAccount,
        password_hash: &PasswordHash,
        changes: &[FieldChange],
    ) -> impl Future<Output = Result<Option<StaffAccount>, StoreError>> + Send;

    /// Saves the role and enabled flag with their audit `changes` in one transaction. With
    /// `end_sessions`, the account's sessions are deleted in that same transaction.
    fn update_staff(
        &self,
        actor: StaffId,
        account: &StaffAccount,
        changes: &[FieldChange],
        end_sessions: bool,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn create_session(
        &self,
        staff: StaffId,
        token: &TokenHash,
        expires_at: OffsetDateTime,
    ) -> impl Future<Output = Result<(), StoreError>> + Send;

    fn find_session(
        &self,
        token: &TokenHash,
    ) -> impl Future<Output = Result<Option<Session>, StoreError>> + Send;
}

/// Port: one-way password hashing.
pub trait PasswordHasher: Send + Sync {
    fn hash_password(
        &self,
        password: &Password,
    ) -> impl Future<Output = Result<PasswordHash, StoreError>> + Send;

    /// Whether `password` matches `hash`.
    fn verify_password(
        &self,
        password: &str,
        hash: &PasswordHash,
    ) -> impl Future<Output = Result<bool, StoreError>> + Send;
}

/// The same answer for an unknown username and a wrong password, so sign-in does not reveal
/// which accounts exist.
fn invalid_credentials() -> AppError {
    AppError::new(ErrorKind::Unauthenticated, "Invalid username or password")
}

impl<S: StaffStore + PasswordHasher> App<S> {
    /// Checks a username and password and opens a new session.
    pub async fn sign_in(&self, username: &str, password: &str) -> AppResult<NewSession> {
        let username = Username::parse(username)?;
        if password.is_empty() {
            return Err(AppError::validation("password", "password is required"));
        }
        let credentials = self
            .store
            .find_credentials(&username)
            .await?
            .ok_or_else(invalid_credentials)?;
        if !self
            .store
            .verify_password(password, &credentials.password_hash)
            .await?
        {
            return Err(invalid_credentials());
        }
        let account = credentials.account;
        if !account.enabled {
            return Err(
                AppError::forbidden("Staff account is disabled").with("username", &username)
            );
        }

        let token = SessionToken::generate();
        let expires_at = OffsetDateTime::now_utc() + SESSION_LIFETIME;
        self.store
            .create_session(account.id, &TokenHash::of(token.as_str()), expires_at)
            .await?;
        Ok(NewSession { token, account })
    }

    /// Resolves a bearer token to the signed-in actor. Unknown and expired tokens, and tokens
    /// of disabled accounts, are rejected.
    pub async fn authenticate(&self, token: &str) -> AppResult<Actor> {
        let session = self
            .store
            .find_session(&TokenHash::of(token))
            .await?
            .filter(|s| s.account.enabled && s.expires_at > OffsetDateTime::now_utc())
            .ok_or_else(AppError::unauthenticated)?;
        Ok(Actor {
            id: session.account.id,
            role: session.account.role,
        })
    }

    pub async fn list_staff(&self, actor: Actor) -> AppResult<Vec<StaffAccount>> {
        crate::authorize(actor, Action::ListStaffAccounts)?;
        Ok(self.store.list_staff().await?)
    }

    pub async fn create_staff(
        &self,
        actor: Actor,
        account: NewStaffAccount,
    ) -> AppResult<StaffAccount> {
        crate::authorize(actor, Action::CreateStaffAccount)?;
        let hash = self.store.hash_password(&account.password).await?;
        self.store
            .create_staff(actor.id, &account, &hash, &account.changes())
            .await
            .map_err(|err| username_taken(err, &account.username))
    }

    /// Changes an account's role and/or enabled flag. Disabling ends its sessions at once.
    pub async fn update_staff(
        &self,
        actor: Actor,
        id: StaffId,
        role: Option<Role>,
        enabled: Option<bool>,
    ) -> AppResult<StaffAccount> {
        crate::authorize(actor, Action::UpdateStaffAccount)?;
        let current = self
            .store
            .find_staff(id)
            .await?
            .ok_or_else(AppError::not_found)?;
        let (updated, changes) = current.update(role, enabled);
        if !changes.is_empty() {
            self.store
                .update_staff(actor.id, &updated, &changes, !updated.enabled)
                .await?;
        }
        Ok(updated)
    }

    /// Creates the first Administrator, only when no staff account exists yet. Returns `None`
    /// when accounts already exist.
    pub async fn bootstrap_admin(
        &self,
        username: Username,
        password: Password,
    ) -> AppResult<Option<StaffAccount>> {
        let account = NewStaffAccount {
            username,
            password,
            role: Role::Administrator,
        };
        let hash = self.store.hash_password(&account.password).await?;
        self.store
            .create_first_staff(&account, &hash, &account.changes())
            .await
            .map_err(|err| username_taken(err, &account.username))
    }
}

fn username_taken(err: StoreError, username: &Username) -> AppError {
    match err {
        StoreError::Duplicate { .. } => {
            AppError::conflict("username already exists").with("username", username)
        }
        other => other.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_are_random_and_url_safe() {
        let a = SessionToken::generate();
        let b = SessionToken::generate();
        assert_ne!(a, b);
        assert_eq!(a.as_str().len(), 43);
        assert!(
            a.as_str()
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        );
    }

    #[test]
    fn token_hash_is_sha256() {
        let hash = TokenHash::of("abc");
        assert_eq!(hash.0[..4], [0xba, 0x78, 0x16, 0xbf]);
        assert_ne!(TokenHash::of("abd"), hash);
    }

    #[test]
    fn secrets_are_not_debug_printed() {
        let token = SessionToken::generate();
        assert!(!format!("{token:?}").contains(token.as_str()));
        let hash = PasswordHash("$argon2id$v=19$secret".into());
        assert!(!format!("{hash:?}").contains("argon2"));
    }
}
