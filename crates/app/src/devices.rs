//! Serviced devices (REQ-F-004).

use fixops_domain::audit::FieldChange;
use fixops_domain::device::{Device, DevicePatch, NewDevice};
use fixops_domain::id::{ClientId, DeviceId, StaffId};
use fixops_domain::staff::{Action, Actor};

use crate::{App, AppError, AppResult, StoreError};

/// Which devices to list. Unset fields match everything.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceFilter {
    pub client_id: Option<ClientId>,
    /// Part of the serial number, in any letter case.
    pub serial_number: Option<String>,
}

/// Port: storage of devices.
pub trait DeviceStore: Send + Sync {
    /// Inserts a device with its audit `changes` in one transaction. An unknown client fails
    /// with [`StoreError::MissingReference`].
    fn create_device(
        &self,
        actor: StaffId,
        device: &NewDevice,
        changes: &[FieldChange],
    ) -> impl Future<Output = Result<Device, StoreError>> + Send;

    fn find_device(
        &self,
        id: DeviceId,
    ) -> impl Future<Output = Result<Option<Device>, StoreError>> + Send;

    /// The device together with its version, read in one statement, for an update.
    fn find_device_with_version(
        &self,
        id: DeviceId,
    ) -> impl Future<Output = Result<Option<(Device, i64)>, StoreError>> + Send;

    /// Matching devices, ordered by id.
    fn list_devices(
        &self,
        filter: &DeviceFilter,
    ) -> impl Future<Output = Result<Vec<Device>, StoreError>> + Send;

    /// Saves every field of `device` with its audit `changes` in one transaction, only while
    /// the stored version still equals `version`, and increments it. When the device was saved
    /// since it was read, nothing is written and the result is [`StoreError::Stale`].
    fn update_device(
        &self,
        actor: StaffId,
        device: &Device,
        version: i64,
        changes: &[FieldChange],
    ) -> impl Future<Output = Result<(), StoreError>> + Send;
}

impl<S: DeviceStore> App<S> {
    /// Creates a device for an existing client.
    pub async fn create_device(&self, actor: Actor, device: NewDevice) -> AppResult<Device> {
        crate::authorize(actor, Action::CreateDevice)?;
        self.store
            .create_device(actor.id, &device, &device.changes())
            .await
            .map_err(|err| match err {
                StoreError::MissingReference { .. } => {
                    AppError::not_found().with("field", "client_id")
                }
                other => other.into(),
            })
    }

    pub async fn get_device(&self, actor: Actor, id: DeviceId) -> AppResult<Device> {
        crate::authorize(actor, Action::GetDevice)?;
        self.load_device(id).await
    }

    pub async fn list_devices(&self, actor: Actor, filter: DeviceFilter) -> AppResult<Vec<Device>> {
        crate::authorize(actor, Action::ListDevices)?;
        Ok(self.store.list_devices(&filter).await?)
    }

    pub async fn update_device(
        &self,
        actor: Actor,
        id: DeviceId,
        patch: DevicePatch,
    ) -> AppResult<Device> {
        crate::authorize(actor, Action::UpdateDevice)?;
        let (current, version) = self
            .store
            .find_device_with_version(id)
            .await?
            .ok_or_else(AppError::not_found)?;
        let (updated, changes) = current.update(patch);
        // A concurrent save in between is `409 GUARD_VIOLATION`.
        if !changes.is_empty() {
            self.store
                .update_device(actor.id, &updated, version, &changes)
                .await?;
        }
        Ok(updated)
    }

    async fn load_device(&self, id: DeviceId) -> AppResult<Device> {
        self.store
            .find_device(id)
            .await?
            .ok_or_else(AppError::not_found)
    }
}
