//! [`DeviceStore`]: serviced devices.

use fixops_app::StoreError;
use fixops_app::devices::{DeviceFilter, DeviceStore};
use fixops_domain::audit::{FieldChange, RecordType};
use fixops_domain::device::{Device, DeviceType, NewDevice, SerialNumber, Symptoms};
use fixops_domain::id::{ClientId, DeviceId, StaffId};
use sqlx::FromRow;

use crate::audit::insert_audit;
use crate::error::{decode_error, store_error};
use crate::{PgStore, contains_pattern};

#[derive(FromRow)]
struct DeviceRow {
    id: i64,
    client_id: i64,
    device_type: String,
    serial_number: String,
    reported_symptoms: String,
}

#[derive(FromRow)]
struct VersionedDeviceRow {
    #[sqlx(flatten)]
    device: DeviceRow,
    version: i64,
}

impl TryFrom<DeviceRow> for Device {
    type Error = sqlx::Error;

    fn try_from(row: DeviceRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: DeviceId(row.id),
            client_id: ClientId(row.client_id),
            device_type: DeviceType::parse(&row.device_type).map_err(decode_error)?,
            serial_number: SerialNumber::parse(&row.serial_number).map_err(decode_error)?,
            reported_symptoms: Symptoms::parse(&row.reported_symptoms).map_err(decode_error)?,
        })
    }
}

impl DeviceStore for PgStore {
    async fn create_device(
        &self,
        actor: StaffId,
        device: &NewDevice,
        changes: &[FieldChange],
    ) -> Result<Device, StoreError> {
        let mut tx = self.pool().begin().await.map_err(store_error)?;
        let created: Device = sqlx::query_as::<_, DeviceRow>(
            "INSERT INTO devices (client_id, device_type, serial_number, reported_symptoms)
             VALUES ($1, $2, $3, $4)
             RETURNING id, client_id, device_type, serial_number, reported_symptoms",
        )
        .bind(device.client_id.0)
        .bind(device.device_type.as_str())
        .bind(device.serial_number.as_str())
        .bind(device.reported_symptoms.as_str())
        .fetch_one(&mut *tx)
        .await
        .and_then(Device::try_from)
        .map_err(store_error)?;
        insert_audit(&mut tx, actor, RecordType::Device, created.id.0, changes)
            .await
            .map_err(store_error)?;
        tx.commit().await.map_err(store_error)?;
        Ok(created)
    }

    async fn find_device(&self, id: DeviceId) -> Result<Option<Device>, StoreError> {
        sqlx::query_as::<_, DeviceRow>(
            "SELECT id, client_id, device_type, serial_number, reported_symptoms
             FROM devices WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(self.pool())
        .await
        .and_then(|row| row.map(Device::try_from).transpose())
        .map_err(store_error)
    }

    async fn list_devices(&self, filter: &DeviceFilter) -> Result<Vec<Device>, StoreError> {
        sqlx::query_as::<_, DeviceRow>(
            "SELECT id, client_id, device_type, serial_number, reported_symptoms
             FROM devices
             WHERE ($1::bigint IS NULL OR client_id = $1)
               AND ($2::text IS NULL OR serial_number ILIKE $2)
             ORDER BY id",
        )
        .bind(filter.client_id.map(|id| id.0))
        .bind(filter.serial_number.as_deref().map(contains_pattern))
        .fetch_all(self.pool())
        .await
        .map_err(store_error)?
        .into_iter()
        .map(|row| Device::try_from(row).map_err(store_error))
        .collect()
    }

    async fn find_device_with_version(
        &self,
        id: DeviceId,
    ) -> Result<Option<(Device, i64)>, StoreError> {
        let row = sqlx::query_as::<_, VersionedDeviceRow>(
            "SELECT id, client_id, device_type, serial_number, reported_symptoms, version
             FROM devices WHERE id = $1",
        )
        .bind(id.0)
        .fetch_optional(self.pool())
        .await
        .map_err(store_error)?;
        row.map(|row| Ok((row.device.try_into().map_err(store_error)?, row.version)))
            .transpose()
    }

    async fn update_device(
        &self,
        actor: StaffId,
        device: &Device,
        version: i64,
        changes: &[FieldChange],
    ) -> Result<(), StoreError> {
        let mut tx = self.pool().begin().await.map_err(store_error)?;
        let updated = sqlx::query(
            "UPDATE devices
             SET device_type = $3, serial_number = $4, reported_symptoms = $5,
                 version = version + 1
             WHERE id = $1 AND version = $2",
        )
        .bind(device.id.0)
        .bind(version)
        .bind(device.device_type.as_str())
        .bind(device.serial_number.as_str())
        .bind(device.reported_symptoms.as_str())
        .execute(&mut *tx)
        .await
        .map_err(store_error)?;
        // Dropping the transaction rolls it back, so a stale save writes nothing.
        if updated.rows_affected() == 0 {
            return Err(StoreError::Stale);
        }
        insert_audit(&mut tx, actor, RecordType::Device, device.id.0, changes)
            .await
            .map_err(store_error)?;
        tx.commit().await.map_err(store_error)
    }
}
