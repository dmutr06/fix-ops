//! [`ClientStore`]: clients and their loyalty tiers.

use fixops_app::StoreError;
use fixops_app::clients::ClientStore;
use fixops_domain::audit::{FieldChange, RecordType};
use fixops_domain::client::{Client, ClientName, Email, LoyaltyTier, NewClient, Phone};
use fixops_domain::id::{ClientId, StaffId};
use sqlx::FromRow;

use crate::audit::insert_audit;
use crate::error::{decode_error, store_error};
use crate::{PgStore, contains_pattern};

#[derive(FromRow)]
struct ClientRow {
    id: i64,
    name: String,
    phone: String,
    email: Option<String>,
    loyalty_tier: String,
}

impl TryFrom<ClientRow> for Client {
    type Error = sqlx::Error;

    fn try_from(row: ClientRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: ClientId(row.id),
            name: ClientName::parse(&row.name).map_err(decode_error)?,
            phone: Phone::parse(&row.phone).map_err(decode_error)?,
            email: row
                .email
                .as_deref()
                .map(Email::parse)
                .transpose()
                .map_err(decode_error)?,
            loyalty_tier: LoyaltyTier::parse(&row.loyalty_tier).map_err(decode_error)?,
        })
    }
}

impl ClientStore for PgStore {
    async fn create_client(
        &self,
        actor: StaffId,
        client: &NewClient,
        changes: &[FieldChange],
    ) -> Result<Client, StoreError> {
        let mut tx = self.pool().begin().await.map_err(store_error)?;
        let created: Client = sqlx::query_as::<_, ClientRow>(
            "INSERT INTO clients (name, phone, email, loyalty_tier)
             VALUES ($1, $2, $3, $4)
             RETURNING id, name, phone, email, loyalty_tier",
        )
        .bind(client.name.as_str())
        .bind(client.phone.as_str())
        .bind(client.email.as_ref().map(Email::as_str))
        .bind(LoyaltyTier::default().as_str())
        .fetch_one(&mut *tx)
        .await
        .and_then(Client::try_from)
        .map_err(store_error)?;
        insert_audit(&mut tx, actor, RecordType::Client, created.id.0, changes)
            .await
            .map_err(store_error)?;
        tx.commit().await.map_err(store_error)?;
        Ok(created)
    }

    async fn find_client(&self, id: ClientId) -> Result<Option<Client>, StoreError> {
        sqlx::query_as::<_, ClientRow>(
            "SELECT id, name, phone, email, loyalty_tier FROM clients WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(self.pool())
        .await
        .and_then(|row| row.map(Client::try_from).transpose())
        .map_err(store_error)
    }

    async fn search_clients(&self, text: Option<&str>) -> Result<Vec<Client>, StoreError> {
        sqlx::query_as::<_, ClientRow>(
            "SELECT id, name, phone, email, loyalty_tier FROM clients
             WHERE $1::text IS NULL OR name ILIKE $1 OR phone ILIKE $1
             ORDER BY id",
        )
        .bind(text.map(contains_pattern))
        .fetch_all(self.pool())
        .await
        .map_err(store_error)?
        .into_iter()
        .map(|row| Client::try_from(row).map_err(store_error))
        .collect()
    }

    async fn update_client(
        &self,
        actor: StaffId,
        client: &Client,
        changes: &[FieldChange],
    ) -> Result<(), StoreError> {
        let mut tx = self.pool().begin().await.map_err(store_error)?;
        sqlx::query(
            "UPDATE clients SET name = $2, phone = $3, email = $4, loyalty_tier = $5
             WHERE id = $1",
        )
        .bind(client.id.0)
        .bind(client.name.as_str())
        .bind(client.phone.as_str())
        .bind(client.email.as_ref().map(Email::as_str))
        .bind(client.loyalty_tier.as_str())
        .execute(&mut *tx)
        .await
        .map_err(store_error)?;
        insert_audit(&mut tx, actor, RecordType::Client, client.id.0, changes)
            .await
            .map_err(store_error)?;
        tx.commit().await.map_err(store_error)
    }
}
