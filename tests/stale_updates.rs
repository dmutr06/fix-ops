//! Client and device updates never overwrite a newer saved state (REQ-F-003, REQ-F-004,
//! REQ-F-017, REQ-NF-003; verification P2 D-1).
//!
//! A stale save is produced deterministically: read a record through the store, change it
//! through the API so its stored state moves on, then save the old copy.

mod common;

use axum::http::StatusCode;
use axum::response::IntoResponse;
use common::TestApp;
use fixops::http::ApiError;
use fixops_app::StoreError;
use fixops_app::clients::ClientStore;
use fixops_app::devices::DeviceStore;
use fixops_domain::client::{ClientName, ClientPatch, LoyaltyTier};
use fixops_domain::device::{DevicePatch, SerialNumber};
use fixops_domain::id::{ClientId, DeviceId, StaffId};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;

struct Setup {
    app: TestApp,
    admin: String,
    admin_id: StaffId,
    client: i64,
    device: i64,
}

async fn setup(pool: PgPool) -> Setup {
    let app = TestApp::new(pool);
    let admin = app.admin_token().await;
    let admin_id = app
        .get("/api/v1/staff")
        .token(&admin)
        .send()
        .await
        .expect(StatusCode::OK)[0]["id"]
        .as_i64()
        .expect("admin id");
    let client = app
        .post("/api/v1/clients")
        .token(&admin)
        .json(&json!({ "name": "Bob Vance", "phone": "+1-555-0100" }))
        .send()
        .await
        .expect(StatusCode::CREATED)["id"]
        .as_i64()
        .expect("client id");
    let device = app
        .post("/api/v1/devices")
        .token(&admin)
        .json(&json!({
            "client_id": client,
            "device_type": "Laptop",
            "serial_number": "SN-THINKPAD-9001",
            "reported_symptoms": "No power on AC adapter"
        }))
        .send()
        .await
        .expect(StatusCode::CREATED)["id"]
        .as_i64()
        .expect("device id");
    Setup {
        app,
        admin,
        admin_id: StaffId(admin_id),
        client,
        device,
    }
}

impl Setup {
    async fn get(&self, path: &str) -> Value {
        self.app
            .get(&format!("/api/v1{path}"))
            .token(&self.admin)
            .send()
            .await
            .expect(StatusCode::OK)
    }

    async fn patch(&self, path: &str, body: &Value) -> Value {
        self.app
            .request(axum::http::Method::PATCH, &format!("/api/v1{path}"))
            .token(&self.admin)
            .json(body)
            .send()
            .await
            .expect(StatusCode::OK)
    }

    async fn audit(&self) -> Value {
        self.get("/audit-logs").await
    }
}

/// The response a stale save produces: `409 GUARD_VIOLATION`.
async fn assert_stale(err: StoreError) {
    assert!(matches!(err, StoreError::Stale), "{err:?}");
    let response = ApiError::from(err).into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["code"], "GUARD_VIOLATION", "{body:#}");
}

/// The last audited value of `field` on the record.
fn last_audited(audit: &Value, record_type: &str, record_id: i64, field: &str) -> Value {
    audit
        .as_array()
        .unwrap()
        .iter()
        .rfind(|e| {
            e["record_type"] == record_type && e["record_id"] == record_id && e["field"] == field
        })
        .map_or_else(
            || panic!("{field} never audited"),
            |e| e["new_value"].clone(),
        )
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn stale_client_update_is_rejected_and_changes_nothing(pool: PgPool) {
    let s = setup(pool).await;
    let (stale, version) = s
        .app
        .app()
        .store()
        .find_client_with_version(ClientId(s.client))
        .await
        .unwrap()
        .unwrap();

    s.patch(
        &format!("/clients/{}", s.client),
        &json!({ "phone": "+1-555-0199" }),
    )
    .await;
    let record = s.get(&format!("/clients/{}", s.client)).await;
    let audit = s.audit().await;

    let (renamed, changes) = stale.update(ClientPatch {
        name: Some(ClientName::parse("Robert Vance").unwrap()),
        ..ClientPatch::default()
    });
    let err = s
        .app
        .app()
        .store()
        .update_client(s.admin_id, &renamed, version, &changes)
        .await
        .expect_err("a stale save must be rejected");
    assert_stale(err).await;

    assert_eq!(s.get(&format!("/clients/{}", s.client)).await, record);
    assert_eq!(record["phone"], "+1-555-0199");
    assert_eq!(s.audit().await, audit);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn stale_tier_change_is_rejected_and_changes_nothing(pool: PgPool) {
    let s = setup(pool).await;
    let (stale, version) = s
        .app
        .app()
        .store()
        .find_client_with_version(ClientId(s.client))
        .await
        .unwrap()
        .unwrap();

    s.patch(
        &format!("/clients/{}", s.client),
        &json!({ "phone": "+1-555-0199" }),
    )
    .await;
    let record = s.get(&format!("/clients/{}", s.client)).await;
    let audit = s.audit().await;

    let (gold, changes) = stale.set_loyalty_tier(LoyaltyTier::Gold);
    let err = s
        .app
        .app()
        .store()
        .update_client(s.admin_id, &gold, version, &changes)
        .await
        .expect_err("a stale tier change must be rejected");
    assert_stale(err).await;

    let after = s.get(&format!("/clients/{}", s.client)).await;
    assert_eq!(after, record);
    assert_eq!(after["loyalty_tier"], "Standard");
    assert_eq!(s.audit().await, audit);
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn stale_device_update_is_rejected_and_changes_nothing(pool: PgPool) {
    let s = setup(pool).await;
    let (stale, version) = s
        .app
        .app()
        .store()
        .find_device_with_version(DeviceId(s.device))
        .await
        .unwrap()
        .unwrap();

    s.patch(
        &format!("/devices/{}", s.device),
        &json!({ "reported_symptoms": "Battery LED blinks 3x" }),
    )
    .await;
    let record = s.get(&format!("/devices/{}", s.device)).await;
    let audit = s.audit().await;

    let (changed, changes) = stale.update(DevicePatch {
        serial_number: Some(SerialNumber::parse("SN-OTHER").unwrap()),
        ..DevicePatch::default()
    });
    let err = s
        .app
        .app()
        .store()
        .update_device(s.admin_id, &changed, version, &changes)
        .await
        .expect_err("a stale save must be rejected");
    assert_stale(err).await;

    let after = s.get(&format!("/devices/{}", s.device)).await;
    assert_eq!(after, record);
    assert_eq!(after["reported_symptoms"], "Battery LED blinks 3x");
    assert_eq!(s.audit().await, audit);
}

/// Saving a freshly read copy each time succeeds, however often the record changed before.
#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn fresh_saves_succeed_one_after_another(pool: PgPool) {
    let s = setup(pool).await;
    let store = s.app.app().store();
    for name in ["Robert Vance", "Bobby Vance"] {
        let (current, version) = store
            .find_client_with_version(ClientId(s.client))
            .await
            .unwrap()
            .unwrap();
        let (renamed, changes) = current.update(ClientPatch {
            name: Some(ClientName::parse(name).unwrap()),
            ..ClientPatch::default()
        });
        store
            .update_client(s.admin_id, &renamed, version, &changes)
            .await
            .expect("fresh save");
    }
    for symptoms in ["Fans loud", "Fans loud; no POST"] {
        let (current, version) = store
            .find_device_with_version(DeviceId(s.device))
            .await
            .unwrap()
            .unwrap();
        let (changed, changes) = current.update(DevicePatch {
            reported_symptoms: Some(fixops_domain::device::Symptoms::parse(symptoms).unwrap()),
            ..DevicePatch::default()
        });
        store
            .update_device(s.admin_id, &changed, version, &changes)
            .await
            .expect("fresh save");
    }
    assert_eq!(
        s.get(&format!("/clients/{}", s.client)).await["name"],
        "Bobby Vance"
    );
    assert_eq!(
        s.get(&format!("/devices/{}", s.device)).await["reported_symptoms"],
        "Fans loud; no POST"
    );
}

/// Sequential API updates all persist, and the last audited value of every field equals the
/// stored value (REQ-NF-003).
#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn sequential_updates_persist_and_match_the_audit_log(pool: PgPool) {
    let s = setup(pool).await;
    let client_path = format!("/clients/{}", s.client);
    let device_path = format!("/devices/{}", s.device);

    s.patch(&client_path, &json!({ "name": "Robert Vance" }))
        .await;
    s.patch(&client_path, &json!({ "phone": "+1-555-0199" }))
        .await;
    s.app
        .put(&format!("/api/v1/clients/{}/loyalty-tier", s.client))
        .token(&s.admin)
        .json(&json!({ "loyalty_tier": "Silver" }))
        .send()
        .await
        .expect(StatusCode::OK);
    s.patch(&client_path, &json!({ "email": "bob@vance.org" }))
        .await;
    s.patch(&device_path, &json!({ "serial_number": "SN-TOWER-1" }))
        .await;
    s.patch(&device_path, &json!({ "device_type": "Desktop" }))
        .await;

    let client = s.get(&client_path).await;
    let device = s.get(&device_path).await;
    assert_eq!(
        client,
        json!({
            "id": s.client,
            "name": "Robert Vance",
            "phone": "+1-555-0199",
            "email": "bob@vance.org",
            "loyalty_tier": "Silver"
        })
    );
    assert_eq!(device["serial_number"], "SN-TOWER-1");
    assert_eq!(device["device_type"], "Desktop");

    let audit = s.audit().await;
    for field in ["name", "phone", "email", "loyalty_tier"] {
        assert_eq!(
            last_audited(&audit, "Client", s.client, field),
            client[field],
            "{field}"
        );
    }
    for field in ["device_type", "serial_number", "reported_symptoms"] {
        assert_eq!(
            last_audited(&audit, "Device", s.device, field),
            device[field],
            "{field}"
        );
    }
}

#[sqlx::test(migrator = "fixops_infra::MIGRATOR")]
async fn stale_staff_role_update_cannot_undo_disable(pool: PgPool) {
    use fixops_app::staff::StaffStore;
    use fixops_domain::staff::Role;
    use fixops_infra::PgStore;

    let s = setup(pool).await;
    let id = s
        .app
        .create_staff(&s.admin, "stale.tech", "Technician")
        .await;
    let token = s.app.sign_in("stale.tech", common::PASSWORD).await;
    let store = PgStore::new(s.app.pool.clone());
    let original = store.find_staff(StaffId(id)).await.unwrap().unwrap();
    let (updated, changes) = original.update(Some(Role::Receptionist), None);
    s.patch(&format!("/staff/{id}"), &json!({ "enabled": false }))
        .await;
    let before = s.audit().await;

    let err = store
        .update_staff(s.admin_id, &updated, &changes, false)
        .await
        .unwrap_err();
    assert_stale(err).await;
    let saved = store.find_staff(StaffId(id)).await.unwrap().unwrap();
    assert_eq!(saved.role, Role::Technician);
    assert!(!saved.enabled);
    assert_eq!(s.audit().await, before);
    s.app
        .get("/api/v1/audit-logs")
        .token(&token)
        .send()
        .await
        .expect_error(StatusCode::UNAUTHORIZED, "UNAUTHENTICATED");
}
