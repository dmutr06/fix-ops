//! `/devices`: serviced device records and search (REQ-F-004).

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use fixops_app::devices::DeviceFilter;
use fixops_app::{AppError, ErrorKind};
use fixops_domain::device::{Device, DevicePatch, DeviceType, NewDevice, SerialNumber, Symptoms};
use fixops_domain::id::{ClientId, DeviceId};
use fixops_domain::staff::Action;
use serde::{Deserialize, Serialize};

use super::auth::SignedIn;
use super::json::{self, Json};
use super::params::{PathId, Query, non_empty, positive_id, record_id};
use super::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/devices", get(list).post(create))
        .route("/devices/{deviceId}", get(show).patch(update))
}

/// Contract `Device`.
#[derive(Debug, Serialize)]
struct DeviceDto {
    id: i64,
    client_id: i64,
    device_type: &'static str,
    serial_number: String,
    reported_symptoms: String,
}

impl From<Device> for DeviceDto {
    fn from(device: Device) -> Self {
        Self {
            id: device.id.0,
            client_id: device.client_id.0,
            device_type: device.device_type.as_str(),
            serial_number: device.serial_number.as_str().to_owned(),
            reported_symptoms: device.reported_symptoms.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceQuery {
    client_id: Option<i64>,
    q: Option<String>,
}

impl DeviceQuery {
    fn into_filter(self) -> Result<DeviceFilter, AppError> {
        Ok(DeviceFilter {
            client_id: positive_id("client_id", self.client_id)?.map(ClientId),
            serial_number: non_empty("q", self.q)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceCreateRequest {
    client_id: i64,
    device_type: String,
    serial_number: String,
    reported_symptoms: String,
}

impl DeviceCreateRequest {
    fn into_new_device(self) -> Result<NewDevice, AppError> {
        Ok(NewDevice {
            client_id: ClientId(record_id("client_id", self.client_id)?),
            device_type: DeviceType::parse(&self.device_type)?,
            serial_number: SerialNumber::parse(&self.serial_number)?,
            reported_symptoms: Symptoms::parse(&self.reported_symptoms)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceUpdateRequest {
    #[serde(default, deserialize_with = "json::not_null")]
    device_type: Option<String>,
    #[serde(default, deserialize_with = "json::not_null")]
    serial_number: Option<String>,
    #[serde(default, deserialize_with = "json::not_null")]
    reported_symptoms: Option<String>,
}

impl DeviceUpdateRequest {
    fn into_patch(self) -> Result<DevicePatch, AppError> {
        if self.device_type.is_none()
            && self.serial_number.is_none()
            && self.reported_symptoms.is_none()
        {
            return Err(AppError::new(
                ErrorKind::Validation,
                "at least one of device_type, serial_number or reported_symptoms is required",
            ));
        }
        Ok(DevicePatch {
            device_type: self
                .device_type
                .as_deref()
                .map(DeviceType::parse)
                .transpose()?,
            serial_number: self
                .serial_number
                .as_deref()
                .map(SerialNumber::parse)
                .transpose()?,
            reported_symptoms: self
                .reported_symptoms
                .as_deref()
                .map(Symptoms::parse)
                .transpose()?,
        })
    }
}

async fn list(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    query: Result<Query<DeviceQuery>, ApiError>,
) -> Result<Json<Vec<DeviceDto>>, ApiError> {
    state.app.authorize(actor, Action::ListDevices)?;
    let Query(query) = query?;
    let devices = state.app.list_devices(actor, query.into_filter()?).await?;
    Ok(Json(devices.into_iter().map(Into::into).collect()))
}

async fn create(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    body: Result<Json<DeviceCreateRequest>, ApiError>,
) -> Result<(StatusCode, Json<DeviceDto>), ApiError> {
    state.app.authorize(actor, Action::CreateDevice)?;
    let Json(body) = body?;
    let created = state
        .app
        .create_device(actor, body.into_new_device()?)
        .await?;
    Ok((StatusCode::CREATED, Json(created.into())))
}

async fn show(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    id: Result<PathId, ApiError>,
) -> Result<Json<DeviceDto>, ApiError> {
    state.app.authorize(actor, Action::GetDevice)?;
    let PathId(id) = id?;
    let device = state.app.get_device(actor, DeviceId(id)).await?;
    Ok(Json(device.into()))
}

async fn update(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    id: Result<PathId, ApiError>,
    body: Result<Json<DeviceUpdateRequest>, ApiError>,
) -> Result<Json<DeviceDto>, ApiError> {
    state.app.authorize(actor, Action::UpdateDevice)?;
    let PathId(id) = id?;
    let Json(body) = body?;
    let updated = state
        .app
        .update_device(actor, DeviceId(id), body.into_patch()?)
        .await?;
    Ok(Json(updated.into()))
}
