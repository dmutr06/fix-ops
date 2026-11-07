//! Client records and loyalty tiers (REQ-F-003, REQ-F-017).

use fixops_domain::audit::FieldChange;
use fixops_domain::client::{Client, ClientPatch, LoyaltyTier, NewClient};
use fixops_domain::id::{ClientId, StaffId};
use fixops_domain::staff::{Action, Actor};

use crate::{App, AppError, AppResult, StoreError};

/// Port: storage of clients.
pub trait ClientStore: Send + Sync {
    /// Inserts a client at the starting tier with its audit `changes` in one transaction.
    fn create_client(
        &self,
        actor: StaffId,
        client: &NewClient,
        changes: &[FieldChange],
    ) -> impl Future<Output = Result<Client, StoreError>> + Send;

    fn find_client(
        &self,
        id: ClientId,
    ) -> impl Future<Output = Result<Option<Client>, StoreError>> + Send;

    /// The client together with its version, read in one statement, for an update.
    fn find_client_with_version(
        &self,
        id: ClientId,
    ) -> impl Future<Output = Result<Option<(Client, i64)>, StoreError>> + Send;

    /// Clients whose name or phone contains `text`, ignoring case; all clients without `text`.
    /// Ordered by id.
    fn search_clients(
        &self,
        text: Option<&str>,
    ) -> impl Future<Output = Result<Vec<Client>, StoreError>> + Send;

    /// Saves every field of `client` with its audit `changes` in one transaction, only while
    /// the stored version still equals `version`, and increments it. When the client was saved
    /// since it was read, nothing is written and the result is [`StoreError::Stale`].
    fn update_client(
        &self,
        actor: StaffId,
        client: &Client,
        version: i64,
        changes: &[FieldChange],
    ) -> impl Future<Output = Result<(), StoreError>> + Send;
}

impl<S: ClientStore> App<S> {
    pub async fn create_client(&self, actor: Actor, client: NewClient) -> AppResult<Client> {
        crate::authorize(actor, Action::CreateClient)?;
        Ok(self
            .store
            .create_client(actor.id, &client, &client.changes())
            .await?)
    }

    pub async fn get_client(&self, actor: Actor, id: ClientId) -> AppResult<Client> {
        crate::authorize(actor, Action::GetClient)?;
        self.load_client(id).await
    }

    /// All clients, or those whose name or phone contains `text` in any letter case.
    pub async fn list_clients(&self, actor: Actor, text: Option<&str>) -> AppResult<Vec<Client>> {
        crate::authorize(actor, Action::ListClients)?;
        Ok(self.store.search_clients(text).await?)
    }

    pub async fn update_client(
        &self,
        actor: Actor,
        id: ClientId,
        patch: ClientPatch,
    ) -> AppResult<Client> {
        crate::authorize(actor, Action::UpdateClient)?;
        let (current, version) = self.load_client_for_update(id).await?;
        let (updated, changes) = current.update(patch);
        self.save_client(actor, &updated, version, &changes).await?;
        Ok(updated)
    }

    /// Sets the tier by hand; nothing else changes a tier (REQ-F-017).
    pub async fn set_loyalty_tier(
        &self,
        actor: Actor,
        id: ClientId,
        tier: LoyaltyTier,
    ) -> AppResult<Client> {
        crate::authorize(actor, Action::SetClientLoyaltyTier)?;
        let (current, version) = self.load_client_for_update(id).await?;
        let (updated, changes) = current.set_loyalty_tier(tier);
        self.save_client(actor, &updated, version, &changes).await?;
        Ok(updated)
    }

    async fn load_client(&self, id: ClientId) -> AppResult<Client> {
        self.store
            .find_client(id)
            .await?
            .ok_or_else(AppError::not_found)
    }

    async fn load_client_for_update(&self, id: ClientId) -> AppResult<(Client, i64)> {
        self.store
            .find_client_with_version(id)
            .await?
            .ok_or_else(AppError::not_found)
    }

    /// Saves `client` if it changed. A concurrent save in between is `409 GUARD_VIOLATION`.
    async fn save_client(
        &self,
        actor: Actor,
        client: &Client,
        version: i64,
        changes: &[FieldChange],
    ) -> AppResult<()> {
        if !changes.is_empty() {
            self.store
                .update_client(actor.id, client, version, changes)
                .await?;
        }
        Ok(())
    }
}
