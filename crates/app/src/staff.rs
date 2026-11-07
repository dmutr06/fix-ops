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
    /// A state that differs from `expected` fails with [`StoreError::Stale`].
    fn update_staff(
        &self,
        actor: StaffId,
        account: &StaffAccount,
        expected: &StaffAccount,
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

    /// Whether `password` matches `hash`. With `None` (no such account) the adapter does the
    /// same hashing work against a dummy hash and answers `false`, so the response time does
    /// not reveal whether a username exists.
    fn verify_password(
        &self,
        password: &str,
        hash: Option<&PasswordHash>,
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
        let credentials = self.store.find_credentials(&username).await?;
        let hash = credentials.as_ref().map(|c| &c.password_hash);
        let verified = self.store.verify_password(password, hash).await?;
        let account = credentials
            .filter(|_| verified)
            .ok_or_else(invalid_credentials)?
            .account;
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
                .update_staff(actor.id, &updated, &current, &changes, !updated.enabled)
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

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the fake ports answer immediately"
    )]
    mod sign_in {
        use std::sync::atomic::{AtomicUsize, Ordering};

        use super::*;

        const PASSWORD: &str = "correct-horse-battery";

        /// One account, `anna`. The fake hash is the password itself; the hasher counts how
        /// often a password is checked.
        #[derive(Default)]
        struct Fake {
            verifications: AtomicUsize,
        }

        impl StaffStore for Fake {
            async fn list_staff(&self) -> Result<Vec<StaffAccount>, StoreError> {
                unreachable!("not used by sign-in")
            }

            async fn find_staff(&self, _: StaffId) -> Result<Option<StaffAccount>, StoreError> {
                unreachable!("not used by sign-in")
            }

            async fn find_credentials(
                &self,
                username: &Username,
            ) -> Result<Option<Credentials>, StoreError> {
                Ok((username.as_str() == "anna").then(|| Credentials {
                    account: StaffAccount {
                        id: StaffId(1),
                        username: username.clone(),
                        role: Role::Receptionist,
                        enabled: true,
                    },
                    password_hash: PasswordHash(PASSWORD.to_owned()),
                }))
            }

            async fn create_staff(
                &self,
                _: StaffId,
                _: &NewStaffAccount,
                _: &PasswordHash,
                _: &[FieldChange],
            ) -> Result<StaffAccount, StoreError> {
                unreachable!("not used by sign-in")
            }

            async fn create_first_staff(
                &self,
                _: &NewStaffAccount,
                _: &PasswordHash,
                _: &[FieldChange],
            ) -> Result<Option<StaffAccount>, StoreError> {
                unreachable!("not used by sign-in")
            }

            async fn update_staff(
                &self,
                _: StaffId,
                _: &StaffAccount,
                _: &StaffAccount,
                _: &[FieldChange],
                _: bool,
            ) -> Result<(), StoreError> {
                unreachable!("not used by sign-in")
            }

            async fn create_session(
                &self,
                _: StaffId,
                _: &TokenHash,
                _: OffsetDateTime,
            ) -> Result<(), StoreError> {
                Ok(())
            }

            async fn find_session(&self, _: &TokenHash) -> Result<Option<Session>, StoreError> {
                unreachable!("not used by sign-in")
            }
        }

        impl PasswordHasher for Fake {
            async fn hash_password(&self, _: &Password) -> Result<PasswordHash, StoreError> {
                unreachable!("not used by sign-in")
            }

            async fn verify_password(
                &self,
                password: &str,
                hash: Option<&PasswordHash>,
            ) -> Result<bool, StoreError> {
                self.verifications.fetch_add(1, Ordering::SeqCst);
                Ok(hash.is_some_and(|hash| hash.0 == password))
            }
        }

        async fn checks_for(username: &str, password: &str) -> (ErrorKind, usize) {
            let app = App::new(Fake::default());
            let err = app.sign_in(username, password).await.unwrap_err();
            (err.kind, app.store().verifications.load(Ordering::SeqCst))
        }

        #[tokio::test]
        async fn unknown_username_costs_the_same_password_check() {
            let wrong_password = checks_for("anna", "not-the-password").await;
            let unknown_user = checks_for("nobody", PASSWORD).await;
            assert_eq!(wrong_password, (ErrorKind::Unauthenticated, 1));
            assert_eq!(unknown_user, wrong_password);
        }

        #[tokio::test]
        async fn correct_password_signs_in() {
            let app = App::new(Fake::default());
            let session = app.sign_in("anna", PASSWORD).await.unwrap();
            assert_eq!(session.account.id, StaffId(1));
        }
    }
}
