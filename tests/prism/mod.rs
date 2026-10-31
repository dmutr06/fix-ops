//! HTTP scenarios against a Prism mock of `spec/openapi.yaml`.
//! Start it with `npx @stoplight/prism-cli@5.16.0 mock spec/openapi.yaml -h 127.0.0.1 -p 4010`.

mod p1_staff_access;
mod p2_intake;
mod p3_diagnosis;
mod p4_repair;
mod p5_settlement;
mod p6_audit;

use std::io::{Read, Write};
use std::net::TcpStream;

use serde_json::Value;

pub const RECEP: &str = "sess_recep_01";
pub const TECH: &str = "sess_tech_01";
pub const ADMIN: &str = "sess_admin_01";

pub struct Req {
    method: &'static str,
    path: &'static str,
    token: Option<&'static str>,
    body: Option<Value>,
    prefer: Option<(u16, &'static str)>,
    invalid: bool,
}

pub fn req(method: &'static str, path: &'static str) -> Req {
    Req {
        method,
        path,
        token: None,
        body: None,
        prefer: None,
        invalid: false,
    }
}

impl Req {
    pub fn token(mut self, token: &'static str) -> Self {
        self.token = Some(token);
        self
    }

    pub fn body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Prism is stateless, so role, existence and lifecycle outcomes are picked by example name.
    pub fn prefer(mut self, code: u16, example: &'static str) -> Self {
        self.prefer = Some((code, example));
        self
    }

    /// The request breaks the spec on purpose; Prism must reject it on its own.
    pub fn invalid(mut self) -> Self {
        self.invalid = true;
        self
    }

    pub fn expect(self, status: u16) -> Value {
        let addr = std::env::var("MOCK_ADDR").unwrap_or_else(|_| "127.0.0.1:4010".into());
        let mut stream = TcpStream::connect(&addr).expect("Prism mock is not running");

        let body = self.body.map(|b| b.to_string()).unwrap_or_default();
        let mut head = format!(
            "{} {} HTTP/1.1\r\nHost: {addr}\r\nAccept: application/json\r\nConnection: close\r\n",
            self.method, self.path
        );
        if let Some(token) = self.token {
            head += &format!("Authorization: Bearer {token}\r\n");
        }
        if let Some((code, example)) = self.prefer {
            head += &format!("Prefer: code={code}, example={example}\r\n");
        }
        if !body.is_empty() {
            head += &format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                body.len()
            );
        }
        write!(stream, "{head}\r\n{body}").unwrap();

        let mut raw = String::new();
        stream.read_to_string(&mut raw).unwrap();
        let (head, body) = raw.split_once("\r\n\r\n").expect("malformed HTTP response");
        let actual: u16 = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        let violations = head.to_ascii_lowercase().contains("\r\nsl-violations:");

        let what = format!("{} {}", self.method, self.path);
        assert_eq!(actual, status, "{what}: {body}");
        assert_eq!(
            violations, self.invalid,
            "{what}: unexpected Prism validation result"
        );

        serde_json::from_str(body).unwrap_or_else(|e| panic!("{what}: bad JSON ({e}): {body}"))
    }
}
