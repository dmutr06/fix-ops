//! [`StaffStore`]: staff accounts and sessions.

use fixops_app::StoreError;
use fixops_app::staff::{Credentials, PasswordHash, Session, StaffStore, TokenHash};
use fixops_domain::audit::{FieldChange, RecordType};
use fixops_domain::id::StaffId;
use fixops_domain::staff::{NewStaffAccount, StaffAccount, Username};
use sqlx::{FromRow, PgConnection};
use time::OffsetDateTime;

use crate::PgStore;
use crate::audit::insert_audit;
use crate::error::{decode_error, store_error};

#[derive(FromRow)]
struct StaffRow {
    id: i64,
    username: String,
    role: String,
    enabled: bool,
}

impl TryFrom<StaffRow> for StaffAccount {
    type Error = sqlx::Error;

    fn try_from(row: StaffRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: StaffId(row.id),
            username: Username::parse(&row.username).map_err(decode_error)?,
            role: row.role.parse().map_err(decode_error)?,
            enabled: row.enabled,
        })
    }
}

#[derive(FromRow)]
struct CredentialsRow {
    #[sqlx(flatten)]
    staff: StaffRow,
    password_hash: String,
}

#[derive(FromRow)]
struct SessionRow {
    #[sqlx(flatten)]
    staff: StaffRow,
    expires_at: OffsetDateTime,
}

/// Inserts an account and its creation audit. `actor` of `None` attributes the creation to
/// the new account itself.
async fn insert_staff(
    conn: &mut PgConnection,
    actor: Option<StaffId>,
    account: &NewStaffAccount,
    password_hash: &PasswordHash,
    changes: &[FieldChange],
) -> Result<StaffAccount, sqlx::Error> {
    let created: StaffAccount = sqlx::query_as::<_, StaffRow>(
        "INSERT INTO staff_accounts (username, password_hash, role)
         VALUES ($1, $2, $3)
         RETURNING id, username, role, enabled",
    )
    .bind(account.username.as_str())
    .bind(&password_hash.0)
    .bind(account.role.as_str())
    .fetch_one(&mut *conn)
    .await?
    .try_into()?;

    let actor = actor.unwrap_or(created.id);
    insert_audit(conn, actor, RecordType::StaffAccount, created.id.0, changes).await?;
    Ok(created)
}

impl StaffStore for PgStore {
    async fn list_staff(&self) -> Result<Vec<StaffAccount>, StoreError> {
        sqlx::query_as::<_, StaffRow>(
            "SELECT id, username, role, enabled FROM staff_accounts ORDER BY id",
        )
        .fetch_all(self.pool())
        .await
        .map_err(store_error)?
        .into_iter()
        .map(|row| StaffAccount::try_from(row).map_err(store_error))
        .collect()
    }

    async fn find_staff(&self, id: StaffId) -> Result<Option<StaffAccount>, StoreError> {
        sqlx::query_as::<_, StaffRow>(
            "SELECT id, username, role, enabled FROM staff_accounts WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(self.pool())
        .await
        .and_then(|row| row.map(StaffAccount::try_from).transpose())
        .map_err(store_error)
    }

    async fn find_credentials(
        &self,
        username: &Username,
    ) -> Result<Option<Credentials>, StoreError> {
        let row = sqlx::query_as::<_, CredentialsRow>(
            "SELECT id, username, role, enabled, password_hash
             FROM staff_accounts WHERE username = $1",
        )
        .bind(username.as_str())
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;

        row.map(|row| {
            Ok(Credentials {
                account: row.staff.try_into().map_err(store_error)?,
                password_hash: PasswordHash(row.password_hash),
            })
        })
        .transpose()
    }

    async fn create_staff(
        &self,
        actor: StaffId,
        account: &NewStaffAccount,
        password_hash: &PasswordHash,
        changes: &[FieldChange],
    ) -> Result<StaffAccount, StoreError> {
        let mut tx = self.pool().begin().await.map_err(store_error)?;
        let created = insert_staff(&mut tx, Some(actor), account, password_hash, changes)
            .await
            .map_err(store_error)?;
        tx.commit().await.map_err(store_error)?;
        Ok(created)
    }

    async fn create_first_staff(
        &self,
        account: &NewStaffAccount,
        password_hash: &PasswordHash,
        changes: &[FieldChange],
    ) -> Result<Option<StaffAccount>, StoreError> {
        let mut tx = self.pool().begin().await.map_err(store_error)?;
        // Serializes concurrent starts, so only one of them can create the first account.
        sqlx::query("LOCK TABLE staff_accounts IN EXCLUSIVE MODE")
            .execute(&mut *tx)
            .await
            .map_err(store_error)?;
        let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM staff_accounts)")
            .fetch_one(&mut *tx)
            .await
            .map_err(store_error)?;
        if exists {
            return Ok(None);
        }
        let created = insert_staff(&mut tx, None, account, password_hash, changes)
            .await
            .map_err(store_error)?;
        tx.commit().await.map_err(store_error)?;
        Ok(Some(created))
    }

    async fn update_staff(
        &self,
        actor: StaffId,
        account: &StaffAccount,
        changes: &[FieldChange],
        end_sessions: bool,
    ) -> Result<(), StoreError> {
        let mut tx = self.pool().begin().await.map_err(store_error)?;
        sqlx::query("UPDATE staff_accounts SET role = $2, enabled = $3 WHERE id = $1")
            .bind(account.id.0)
            .bind(account.role.as_str())
            .bind(account.enabled)
            .execute(&mut *tx)
            .await
            .map_err(store_error)?;
        if end_sessions {
            sqlx::query("DELETE FROM sessions WHERE staff_id = $1")
                .bind(account.id.0)
                .execute(&mut *tx)
                .await
                .map_err(store_error)?;
        }
        insert_audit(
            &mut tx,
            actor,
            RecordType::StaffAccount,
            account.id.0,
            changes,
        )
        .await
        .map_err(store_error)?;
        tx.commit().await.map_err(store_error)
    }

    async fn create_session(
        &self,
        staff: StaffId,
        token: &TokenHash,
        expires_at: OffsetDateTime,
    ) -> Result<(), StoreError> {
        let mut tx = self.pool().begin().await.map_err(store_error)?;
        // Drop this account's expired sessions so the table does not grow without bound.
        sqlx::query("DELETE FROM sessions WHERE staff_id = $1 AND expires_at <= now()")
            .bind(staff.0)
            .execute(&mut *tx)
            .await
            .map_err(store_error)?;
        sqlx::query("INSERT INTO sessions (token_hash, staff_id, expires_at) VALUES ($1, $2, $3)")
            .bind(&token.0[..])
            .bind(staff.0)
            .bind(expires_at)
            .execute(&mut *tx)
            .await
            .map_err(store_error)?;
        tx.commit().await.map_err(store_error)
    }

    async fn find_session(&self, token: &TokenHash) -> Result<Option<Session>, StoreError> {
        let row = sqlx::query_as::<_, SessionRow>(
            "SELECT a.id, a.username, a.role, a.enabled, s.expires_at
             FROM sessions s JOIN staff_accounts a ON a.id = s.staff_id
             WHERE s.token_hash = $1",
        )
        .bind(&token.0[..])
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;

        row.map(|row| {
            Ok(Session {
                account: row.staff.try_into().map_err(store_error)?,
                expires_at: row.expires_at,
            })
        })
        .transpose()
    }
}
