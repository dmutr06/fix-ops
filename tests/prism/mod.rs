mod p1_staff_access;
mod p2_intake;
mod p3_diagnosis;
mod p4_repair;
mod p5_settlement;
mod p6_audit;

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;

use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

pub const RECEP_TOKEN: &str = "sess_recep_01";
pub const TECH_TOKEN: &str = "sess_tech_01";
pub const ADMIN_TOKEN: &str = "sess_admin_01";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockMode {
    /// Request passes Prism schema/security checks; Prism serves the operation's default 2xx example.
    ContractDefault,
    /// Request intentionally violates OpenAPI security or parameter/body schema; Prism rejects it automatically (`sl-violations` header).
    AutoValidation,
    /// Stateless mock cannot evaluate roles, DB uniqueness, or lifecycle/payment state; selects a documented response example via `Prefer`.
    PreferExample {
        code: u16,
        example: Option<&'static str>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum Role {
    Receptionist,
    Technician,
    Administrator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum LoyaltyTier {
    Standard,
    Silver,
    Gold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum DeviceType {
    Desktop,
    Laptop,
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum OrderStatus {
    Received,
    Diagnosing,
    InProgress,
    Ready,
    Completed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum DecisionType {
    Approved,
    Declined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum AuditRecordType {
    StaffAccount,
    Client,
    Device,
    RepairOrder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    Unauthenticated,
    Forbidden,
    ValidationError,
    NotFound,
    Conflict,
    GuardViolation,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ErrorResponse {
    pub code: ErrorCode,
    pub message: String,
    pub details: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StaffAccount {
    pub id: u64,
    pub username: String,
    pub role: Role,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SessionResponse {
    pub token: String,
    pub staff: StaffAccount,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Client {
    pub id: u64,
    pub name: String,
    pub phone: String,
    pub email: Option<String>,
    pub loyalty_tier: LoyaltyTier,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Device {
    pub id: u64,
    pub client_id: u64,
    pub device_type: DeviceType,
    pub serial_number: String,
    pub reported_symptoms: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Estimate {
    pub parts_cents: u64,
    pub labor_cents: u64,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClientDecision {
    pub decision: DecisionType,
    pub decided_at: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PaymentRecord {
    pub amount_cents: u64,
    pub paid_at: String,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PickupRecord {
    pub picked_up_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepairOrder {
    pub id: u64,
    pub ticket_number: String,
    pub client_id: u64,
    pub device_id: u64,
    pub assigned_technician_id: u64,
    pub status: OrderStatus,
    pub diagnostic_findings: Option<String>,
    pub estimate: Option<Estimate>,
    pub client_decision: Option<ClientDecision>,
    pub used_parts_cents: u64,
    pub completed_labor_cents: u64,
    pub repair_checked: bool,
    pub labor_discount_cents: u64,
    pub settlement_cents: u64,
    pub payment: Option<PaymentRecord>,
    pub pickup: Option<PickupRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditEntry {
    pub id: u64,
    pub actor_id: u64,
    pub recorded_at: String,
    pub record_type: AuditRecordType,
    pub record_id: u64,
    pub field: String,
    pub previous_value: Option<String>,
    pub new_value: Option<String>,
}

pub struct HttpRequest<'a> {
    method: &'a str,
    path: &'a str,
    mode: MockMode,
    token: Option<&'a str>,
    body: Option<Value>,
}

pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub raw_body: String,
}

impl HttpResponse {
    pub fn json<T: DeserializeOwned>(&self) -> T {
        serde_json::from_str(&self.raw_body).unwrap_or_else(|err| {
            panic!(
                "response body failed contract schema deserialization ({err}): {}",
                self.raw_body
            )
        })
    }
}

pub fn http(method: &'static str, path: &'static str, mode: MockMode) -> HttpRequest<'static> {
    HttpRequest {
        method,
        path,
        mode,
        token: None,
        body: None,
    }
}

impl<'a> HttpRequest<'a> {
    pub fn bearer(mut self, token: &'a str) -> Self {
        self.token = Some(token);
        self
    }

    pub fn json(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    pub fn send_expect(self, expected_status: u16) -> HttpResponse {
        let addr = std::env::var("MOCK_ADDR").unwrap_or_else(|_| "127.0.0.1:4010".to_string());
        let mut stream = TcpStream::connect(&addr).unwrap_or_else(|err| {
            panic!("failed to connect to Prism HTTP mock at {addr}: {err}")
        });

        let payload = self.body.as_ref().map(Value::to_string).unwrap_or_default();
        let mut raw_req = format!(
            "{} {} HTTP/1.1\r\nHost: {addr}\r\nAccept: application/json\r\nConnection: close\r\n",
            self.method, self.path
        );

        if let Some(tok) = self.token {
            raw_req.push_str(&format!("Authorization: Bearer {tok}\r\n"));
        }
        if let MockMode::PreferExample { code, example } = self.mode {
            match example {
                Some(ex) => raw_req.push_str(&format!("Prefer: code={code}, example={ex}\r\n")),
                None => raw_req.push_str(&format!("Prefer: code={code}\r\n")),
            }
        }
        if !payload.is_empty() {
            raw_req.push_str("Content-Type: application/json\r\n");
            raw_req.push_str(&format!("Content-Length: {}\r\n", payload.len()));
        }
        raw_req.push_str("\r\n");
        raw_req.push_str(&payload);

        stream
            .write_all(raw_req.as_bytes())
            .expect("failed to write HTTP request");
        stream.flush().expect("failed to flush HTTP request");

        let mut reader = BufReader::new(stream);
        let mut status_line = String::new();
        reader
            .read_line(&mut status_line)
            .expect("failed to read HTTP status line");
        let status: u16 = status_line
            .split_whitespace()
            .nth(1)
            .expect("missing HTTP status code")
            .parse()
            .expect("invalid HTTP status code");

        let mut headers = HashMap::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).expect("failed to read header");
            if line == "\r\n" || line == "\n" || line.is_empty() {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
            }
        }

        let raw_body = if let Some(len_str) = headers.get("content-length") {
            let len: usize = len_str.parse().expect("invalid content-length");
            let mut buf = vec![0u8; len];
            reader
                .read_exact(&mut buf)
                .expect("failed to read response body");
            String::from_utf8(buf).expect("response body is not valid UTF-8")
        } else {
            let mut buf = String::new();
            reader
                .read_to_string(&mut buf)
                .expect("failed to read response body to EOF");
            buf
        };

        assert_eq!(
            status, expected_status,
            "{} {} returned HTTP {status} (expected {expected_status}): {raw_body}",
            self.method, self.path
        );

        let has_violations = headers.contains_key("sl-violations");
        match self.mode {
            MockMode::AutoValidation => assert!(
                has_violations,
                "expected Prism automatic validation (`sl-violations` header) on {} {}",
                self.method, self.path
            ),
            MockMode::ContractDefault | MockMode::PreferExample { .. } => assert!(
                !has_violations,
                "unexpected request validation violation on {} {}: {:?}",
                self.method,
                self.path,
                headers.get("sl-violations")
            ),
        }

        let mode_label = match self.mode {
            MockMode::ContractDefault => "CONTRACT-DEFAULT",
            MockMode::AutoValidation => "AUTO-VALIDATION",
            MockMode::PreferExample { .. } => "PREFER-EXAMPLE",
        };
        println!(
            "[{mode_label:<16}] {:<6} {:<46} -> {status}",
            self.method, self.path
        );

        HttpResponse {
            status,
            headers,
            raw_body,
        }
    }
}
