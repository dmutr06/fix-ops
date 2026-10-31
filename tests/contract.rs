//! Checks `spec/openapi.yaml` against the SRS without any server.

use std::collections::BTreeSet;

use serde_json::{Value, json};

fn spec() -> Value {
    let text = std::fs::read_to_string("spec/openapi.yaml").unwrap();
    serde_yaml::from_str::<openapiv3::OpenAPI>(&text).expect("not a valid OpenAPI 3.0 document");
    serde_yaml::from_str(&text).unwrap()
}

fn check_refs(node: &Value, spec: &Value) {
    match node {
        Value::Object(map) => {
            if let Some(r) = map.get("$ref").and_then(Value::as_str) {
                assert!(
                    spec.pointer(r.trim_start_matches('#')).is_some(),
                    "broken $ref {r}"
                );
            }
            map.values().for_each(|v| check_refs(v, spec));
        }
        Value::Array(items) => items.iter().for_each(|v| check_refs(v, spec)),
        _ => {}
    }
}

#[test]
fn refs_resolve() {
    let spec = spec();
    check_refs(&spec, &spec);
}

#[test]
fn every_operation_traces_to_requirements() {
    let spec = spec();
    let mut ids = BTreeSet::new();
    let mut reqs = BTreeSet::new();

    for item in spec["paths"].as_object().unwrap().values() {
        for method in ["get", "post", "put", "patch", "delete"] {
            let Some(op) = item.get(method) else { continue };
            let id = op["operationId"].as_str().expect("missing operationId");
            assert!(ids.insert(id), "duplicate operationId {id}");

            let op_reqs = op["x-requirements"].as_array();
            let op_reqs = op_reqs.unwrap_or_else(|| panic!("{id}: no x-requirements"));
            reqs.extend(op_reqs.iter().map(|r| r.as_str().unwrap()));
        }
    }

    assert_eq!(ids.len(), 26);
    for n in 1..=19 {
        assert!(
            reqs.contains(format!("REQ-F-{n:03}").as_str()),
            "REQ-F-{n:03} not covered"
        );
    }
    for n in 1..=3 {
        assert!(
            reqs.contains(format!("REQ-NF-{n:03}").as_str()),
            "REQ-NF-{n:03} not covered"
        );
    }
}

#[test]
fn error_model_and_money_types() {
    let spec = spec();
    let schemas = &spec["components"]["schemas"];

    assert_eq!(
        schemas["ErrorResponse"]["required"],
        json!(["code", "message"])
    );

    let order = &schemas["RepairOrder"]["properties"];
    for field in [
        "used_parts_cents",
        "completed_labor_cents",
        "labor_discount_cents",
        "settlement_cents",
    ] {
        assert_eq!(
            order[field]["type"], "integer",
            "{field} must be whole cents"
        );
    }
}
