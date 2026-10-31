mod contract;
mod p1_staff_access;
mod p2_intake;
mod p3_diagnosis;
mod p4_repair;
mod p5_settlement;
mod p6_audit;

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::sync::OnceLock;

use openapiv3::OpenAPI;
use serde_json::{Value, json};

pub const RECEP_TOKEN: &str = "sess_recep_01";
pub const TECH_TOKEN: &str = "sess_tech_01";
pub const ADMIN_TOKEN: &str = "sess_admin_01";

pub struct SpecMock {
    pub spec: OpenAPI,
    pub raw: Value,
    tokens: HashMap<&'static str, &'static str>,
}

pub struct MockRequest<'a> {
    mock: &'a SpecMock,
    method: &'a str,
    path: &'a str,
    token: Option<&'a str>,
    body: Option<Value>,
    mock_status: Option<u16>,
    mock_example: Option<&'a str>,
}

#[derive(Debug)]
pub struct MockResponse {
    pub status: u16,
    pub body: Value,
}

pub fn mock() -> &'static SpecMock {
    static INSTANCE: OnceLock<SpecMock> = OnceLock::new();
    INSTANCE.get_or_init(|| SpecMock::from_file("spec/openapi.yaml"))
}

impl SpecMock {
    pub fn from_file(path: &str) -> Self {
        let content = fs::read_to_string(path).expect("failed to read OpenAPI spec");
        let spec: OpenAPI =
            serde_yaml::from_str(&content).expect("OpenAPI 3.0.3 schema validation failed");
        let raw: Value = serde_yaml::from_str(&content).expect("failed to parse YAML as Value");

        let tokens = HashMap::from([
            (RECEP_TOKEN, "Receptionist"),
            (TECH_TOKEN, "Technician"),
            (ADMIN_TOKEN, "Administrator"),
        ]);

        Self { spec, raw, tokens }
    }

    pub fn request<'a>(&'a self, method: &'a str, path: &'a str) -> MockRequest<'a> {
        MockRequest {
            mock: self,
            method,
            path,
            token: None,
            body: None,
            mock_status: None,
            mock_example: None,
        }
    }

    pub fn resolve_ref<'a>(&'a self, node: &'a Value) -> &'a Value {
        if let Some(reference) = node.get("$ref").and_then(Value::as_str) {
            let pointer = reference.trim_start_matches('#');
            let resolved = self
                .raw
                .pointer(pointer)
                .unwrap_or_else(|| panic!("unresolved $ref: {reference}"));
            return self.resolve_ref(resolved);
        }
        node
    }

    fn match_operation<'a>(&'a self, method: &str, full_path: &str) -> Option<&'a Value> {
        let route_only = full_path.split('?').next().unwrap_or(full_path);
        let req_segments: Vec<&str> = route_only.split('/').collect();

        let paths = self.raw.get("paths")?.as_object()?;
        for (pattern, path_item) in paths {
            let pat_segments: Vec<&str> = pattern.split('/').collect();
            if pat_segments.len() == req_segments.len()
                && pat_segments
                    .iter()
                    .zip(&req_segments)
                    .all(|(p, r)| (p.starts_with('{') && p.ends_with('}')) || p == r)
            {
                return path_item.get(method.to_ascii_lowercase());
            }
        }
        None
    }

    fn validate_body(
        &self,
        body: Option<&Value>,
        req_body_spec: &Value,
    ) -> Option<(String, String)> {
        let resolved_req = self.resolve_ref(req_body_spec);
        let schema = self.resolve_ref(&resolved_req["content"]["application/json"]["schema"]);
        let Some(obj) = body.and_then(Value::as_object) else {
            return Some(("request body must be a JSON object".into(), "body".into()));
        };

        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for field in required.iter().filter_map(Value::as_str) {
                match obj.get(field) {
                    None | Some(Value::Null) => {
                        return Some((format!("{field} is required"), field.into()));
                    }
                    Some(Value::String(s)) if s.trim().is_empty() => {
                        return Some((
                            format!("{field} is required and cannot be empty"),
                            field.into(),
                        ));
                    }
                    _ => {}
                }
            }
        }

        if let Some(min_props) = schema.get("minProperties").and_then(Value::as_u64) {
            if (obj.len() as u64) < min_props {
                return Some((
                    format!("at least {min_props} properties are required"),
                    "body".into(),
                ));
            }
        }

        if let Some(props) = schema.get("properties").and_then(Value::as_object) {
            if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                for key in obj.keys() {
                    if !props.contains_key(key) {
                        return Some((format!("unknown property {key}"), key.clone()));
                    }
                }
            }
            for (key, prop) in props {
                let Some(val) = obj.get(key) else { continue };
                if val.is_null() {
                    continue;
                }
                let p = self.resolve_ref(prop);
                if let Some(variants) = p.get("enum").and_then(Value::as_array) {
                    if !variants.contains(val) {
                        return Some((format!("invalid enum value for {key}"), key.clone()));
                    }
                }
                if p.get("type").and_then(Value::as_str) == Some("integer") {
                    let Some(n) = val.as_i64() else {
                        return Some((format!("{key} must be an integer"), key.clone()));
                    };
                    if let Some(min) = p.get("minimum").and_then(Value::as_i64) {
                        if n < min {
                            return Some((format!("{key} must be >= {min}"), key.clone()));
                        }
                    }
                }
            }
        }
        None
    }

    fn extract_response(&self, resp_spec: &Value, example_name: Option<&str>) -> Value {
        let resolved = self.resolve_ref(resp_spec);
        let media = &resolved["content"]["application/json"];
        if let Some(examples) = media.get("examples").and_then(Value::as_object) {
            if let Some(name) = example_name {
                if let Some(ex) = examples.get(name) {
                    return ex["value"].clone();
                }
            }
            if let Some((_, first)) = examples.iter().next() {
                return first["value"].clone();
            }
        }
        self.default_from_schema(&media["schema"])
    }

    fn default_from_schema(&self, schema: &Value) -> Value {
        let s = self.resolve_ref(schema);
        if let Some(first) = s
            .get("allOf")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
        {
            return self.default_from_schema(first);
        }
        match s.get("type").and_then(Value::as_str) {
            Some("object") => {
                let mut map = serde_json::Map::new();
                let required: BTreeSet<&str> = s
                    .get("required")
                    .and_then(Value::as_array)
                    .map(|arr| arr.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                if let Some(props) = s.get("properties").and_then(Value::as_object) {
                    for (k, v) in props {
                        if required.contains(k.as_str()) {
                            map.insert(k.clone(), self.default_from_schema(v));
                        } else {
                            map.insert(k.clone(), Value::Null);
                        }
                    }
                }
                Value::Object(map)
            }
            Some("array") => Value::Array(vec![self.default_from_schema(&s["items"])]),
            Some("integer") => json!(s.get("minimum").and_then(Value::as_i64).unwrap_or(1)),
            Some("boolean") => json!(false),
            Some("string") => s
                .get("enum")
                .and_then(Value::as_array)
                .and_then(|a| a.first())
                .cloned()
                .unwrap_or_else(|| json!("mock-string")),
            _ => Value::Null,
        }
    }
}

impl<'a> MockRequest<'a> {
    pub fn bearer(mut self, token: &'a str) -> Self {
        self.token = Some(token);
        self
    }

    pub fn json(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    pub fn expect_contract_case(mut self, status: u16, example: &'a str) -> Self {
        self.mock_status = Some(status);
        self.mock_example = Some(example);
        self
    }

    pub fn send(self) -> MockResponse {
        let Some(op) = self.mock.match_operation(self.method, self.path) else {
            let not_found = &self.mock.raw["components"]["responses"]["NotFound"];
            return MockResponse {
                status: 404,
                body: self.mock.extract_response(not_found, None),
            };
        };

        let requires_auth = op.get("security") != Some(&json!([]));
        let role = self.token.and_then(|t| self.mock.tokens.get(t)).copied();

        if requires_auth && role.is_none() {
            return MockResponse {
                status: 401,
                body: self.mock.extract_response(&op["responses"]["401"], None),
            };
        }

        if !requires_auth
            && self
                .body
                .as_ref()
                .and_then(|b| b.get("username"))
                .and_then(Value::as_str)
                == Some("disabled.user")
        {
            return MockResponse {
                status: 403,
                body: self.mock.extract_response(&op["responses"]["403"], None),
            };
        }

        if let Some(allowed_roles) = op.get("x-roles").and_then(Value::as_array) {
            let role_str = role.unwrap_or_default();
            if !allowed_roles.iter().any(|r| r.as_str() == Some(role_str)) {
                return MockResponse {
                    status: 403,
                    body: self
                        .mock
                        .extract_response(&op["responses"]["403"], self.mock_example),
                };
            }
        }

        if let Some(req_body) = op.get("requestBody") {
            if let Some((msg, field)) = self.mock.validate_body(self.body.as_ref(), req_body) {
                return MockResponse {
                    status: 400,
                    body: json!({
                        "code": "VALIDATION_ERROR",
                        "message": msg,
                        "details": { "field": field }
                    }),
                };
            }
        }

        if let Some(code) = self.mock_status {
            let code_key = code.to_string();
            return MockResponse {
                status: code,
                body: self
                    .mock
                    .extract_response(&op["responses"][&code_key], self.mock_example),
            };
        }

        let responses = op["responses"].as_object().expect("missing responses");
        let (code_str, resp_spec) = responses
            .iter()
            .find(|(k, _)| k.starts_with('2'))
            .expect("missing 2xx response");

        MockResponse {
            status: code_str.parse().unwrap(),
            body: self.mock.extract_response(resp_spec, self.mock_example),
        }
    }
}
