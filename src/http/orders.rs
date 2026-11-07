//! `/orders`: repair-order intake and reading (REQ-F-005).

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, put};
use fixops_app::AppError;
use fixops_app::orders::{OrderFilter, OrderIntake, OrderView};
use fixops_domain::id::{ClientId, DeviceId, OrderId, StaffId};
use fixops_domain::money::Cents;
use fixops_domain::order::{
    ClientDecision, Decision, DiagnosticFindings, Estimate, OrderStatus, RepairStage, RepairWork,
    TicketNumber,
};
use fixops_domain::staff::Action;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use super::auth::SignedIn;
use super::json::Json;
use super::params::{PathId, Query, non_empty, positive_id, record_id};
use super::{ApiError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/orders", get(list).post(create))
        .route("/orders/{orderId}", get(show))
        .route("/orders/{orderId}/diagnosis", put(diagnosis))
        .route("/orders/{orderId}/estimate", put(estimate))
        .route("/orders/{orderId}/decision", put(decision))
        .route("/orders/{orderId}/repair", put(repair))
}

/// Contract `RepairOrder`. Every field is always present; unrecorded values are `null`.
#[derive(Debug, Serialize)]
struct RepairOrderDto {
    id: i64,
    ticket_number: String,
    client_id: i64,
    device_id: i64,
    assigned_technician_id: i64,
    status: &'static str,
    diagnostic_findings: Option<String>,
    estimate: Option<EstimateDto>,
    client_decision: Option<ClientDecisionDto>,
    used_parts_cents: i64,
    completed_labor_cents: i64,
    repair_checked: bool,
    labor_discount_cents: i64,
    settlement_cents: i64,
    payment: Option<PaymentDto>,
    pickup: Option<PickupDto>,
}

#[derive(Debug, Serialize)]
struct EstimateDto {
    parts_cents: i64,
    labor_cents: i64,
}

#[derive(Debug, Serialize)]
struct ClientDecisionDto {
    decision: &'static str,
    #[serde(with = "time::serde::rfc3339")]
    decided_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
struct PaymentDto {
    amount_cents: i64,
    #[serde(with = "time::serde::rfc3339")]
    paid_at: OffsetDateTime,
}

#[derive(Debug, Serialize)]
struct PickupDto {
    #[serde(with = "time::serde::rfc3339")]
    picked_up_at: OffsetDateTime,
}

impl From<OrderView> for RepairOrderDto {
    fn from(OrderView { order, settlement }: OrderView) -> Self {
        Self {
            id: order.id.0,
            ticket_number: order.ticket_number.as_str().to_owned(),
            client_id: order.client_id.0,
            device_id: order.device_id.0,
            assigned_technician_id: order.assigned_technician_id.0,
            status: order.status.as_str(),
            diagnostic_findings: order
                .diagnostic_findings
                .map(|findings| findings.as_str().to_owned()),
            estimate: order.estimate.map(|estimate| EstimateDto {
                parts_cents: estimate.parts.get(),
                labor_cents: estimate.labor.get(),
            }),
            client_decision: order.client_decision.map(|decision| ClientDecisionDto {
                decision: decision.decision.as_str(),
                decided_at: decision.decided_at,
            }),
            used_parts_cents: order.used_parts.get(),
            completed_labor_cents: order.completed_labor.get(),
            repair_checked: order.repair_checked,
            labor_discount_cents: settlement.labor_discount.get(),
            settlement_cents: settlement.total.get(),
            payment: order.payment.map(|payment| PaymentDto {
                amount_cents: payment.amount.get(),
                paid_at: payment.paid_at,
            }),
            pickup: order
                .picked_up_at
                .map(|picked_up_at| PickupDto { picked_up_at }),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OrderQuery {
    status: Option<String>,
    client_id: Option<i64>,
    ticket_number: Option<String>,
}

impl OrderQuery {
    fn into_filter(self) -> Result<OrderFilter, AppError> {
        Ok(OrderFilter {
            status: self.status.as_deref().map(OrderStatus::parse).transpose()?,
            client_id: positive_id("client_id", self.client_id)?.map(ClientId),
            ticket_number: non_empty("ticket_number", self.ticket_number)?,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairOrderCreateRequest {
    ticket_number: String,
    client_id: i64,
    device_id: i64,
    assigned_technician_id: i64,
}

impl RepairOrderCreateRequest {
    fn into_intake(self) -> Result<OrderIntake, AppError> {
        Ok(OrderIntake {
            ticket_number: TicketNumber::parse(&self.ticket_number)?,
            client_id: ClientId(record_id("client_id", self.client_id)?),
            device_id: DeviceId(record_id("device_id", self.device_id)?),
            assigned_technician_id: StaffId(record_id(
                "assigned_technician_id",
                self.assigned_technician_id,
            )?),
        })
    }
}

async fn list(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    query: Result<Query<OrderQuery>, ApiError>,
) -> Result<Json<Vec<RepairOrderDto>>, ApiError> {
    state.app.authorize(actor, Action::ListRepairOrders)?;
    let Query(query) = query?;
    let orders = state.app.list_orders(actor, query.into_filter()?).await?;
    Ok(Json(orders.into_iter().map(Into::into).collect()))
}

async fn create(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    body: Result<Json<RepairOrderCreateRequest>, ApiError>,
) -> Result<(StatusCode, Json<RepairOrderDto>), ApiError> {
    state.app.authorize(actor, Action::CreateRepairOrder)?;
    let Json(body) = body?;
    let created = state.app.create_order(actor, body.into_intake()?).await?;
    Ok((StatusCode::CREATED, Json(created.into())))
}

async fn show(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    id: Result<PathId, ApiError>,
) -> Result<Json<RepairOrderDto>, ApiError> {
    state.app.authorize(actor, Action::GetRepairOrder)?;
    let PathId(id) = id?;
    let order = state.app.get_order(actor, OrderId(id)).await?;
    Ok(Json(order.into()))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiagnosisRequest {
    diagnostic_findings: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EstimateRequest {
    parts_cents: i64,
    labor_cents: i64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DecisionRequest {
    decision: String,
    #[serde(with = "time::serde::rfc3339")]
    decided_at: OffsetDateTime,
}

async fn diagnosis(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    id: Result<PathId, ApiError>,
    body: Result<Json<DiagnosisRequest>, ApiError>,
) -> Result<Json<RepairOrderDto>, ApiError> {
    state.app.authorize(actor, Action::RecordDiagnosis)?;
    let PathId(id) = id?;
    let Json(body) = body?;
    let findings = body
        .diagnostic_findings
        .as_deref()
        .map(DiagnosticFindings::parse)
        .transpose()
        .map_err(AppError::from)?;
    Ok(Json(
        state
            .app
            .record_diagnosis(actor, OrderId(id), findings)
            .await?
            .into(),
    ))
}

async fn estimate(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    id: Result<PathId, ApiError>,
    body: Result<Json<EstimateRequest>, ApiError>,
) -> Result<Json<RepairOrderDto>, ApiError> {
    state.app.authorize(actor, Action::RecordEstimate)?;
    let PathId(id) = id?;
    let Json(body) = body?;
    let estimate = Estimate {
        parts: Cents::new("parts_cents", body.parts_cents).map_err(AppError::from)?,
        labor: Cents::new("labor_cents", body.labor_cents).map_err(AppError::from)?,
    };
    Ok(Json(
        state
            .app
            .record_estimate(actor, OrderId(id), estimate)
            .await?
            .into(),
    ))
}

async fn decision(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    id: Result<PathId, ApiError>,
    body: Result<Json<DecisionRequest>, ApiError>,
) -> Result<Json<RepairOrderDto>, ApiError> {
    state.app.authorize(actor, Action::RecordClientDecision)?;
    let PathId(id) = id?;
    let Json(body) = body?;
    let decision = ClientDecision {
        decision: Decision::parse(&body.decision).map_err(AppError::from)?,
        decided_at: body.decided_at,
    };
    Ok(Json(
        state
            .app
            .record_decision(actor, OrderId(id), decision)
            .await?
            .into(),
    ))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RepairWorkRequest {
    status: String,
    #[serde(default, deserialize_with = "super::json::not_null")]
    used_parts_cents: Option<i64>,
    #[serde(default, deserialize_with = "super::json::not_null")]
    completed_labor_cents: Option<i64>,
    #[serde(default, deserialize_with = "super::json::not_null")]
    repair_checked: Option<bool>,
}

async fn repair(
    State(state): State<AppState>,
    SignedIn(actor): SignedIn,
    id: Result<PathId, ApiError>,
    body: Result<Json<RepairWorkRequest>, ApiError>,
) -> Result<Json<RepairOrderDto>, ApiError> {
    state.app.authorize(actor, Action::UpdateRepairWork)?;
    let PathId(id) = id?;
    let Json(body) = body?;
    let work = RepairWork {
        stage: RepairStage::parse(&body.status).map_err(AppError::from)?,
        used_parts: body
            .used_parts_cents
            .map(|n| Cents::new("used_parts_cents", n))
            .transpose()
            .map_err(AppError::from)?,
        completed_labor: body
            .completed_labor_cents
            .map(|n| Cents::new("completed_labor_cents", n))
            .transpose()
            .map_err(AppError::from)?,
        repair_checked: body.repair_checked,
    };
    Ok(Json(
        state
            .app
            .update_repair_work(actor, OrderId(id), work)
            .await?
            .into(),
    ))
}
