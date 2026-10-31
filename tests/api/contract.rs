use std::collections::BTreeSet;

use openapiv3::{ReferenceOr, Schema, SchemaKind, Type};
use serde_json::Value;

use crate::{SpecMock, mock};

#[test]
fn openapi_spec_is_valid_and_traceable() {
    let m = mock();

    assert_eq!(m.spec.openapi, "3.0.3");
    assert_eq!(m.spec.info.title, "FixOps Workshop API");
    assert_eq!(m.spec.info.version, "1.0.2");

    let components = m.spec.components.as_ref().expect("missing components");
    assert_eq!(components.schemas.len(), 34);

    let ReferenceOr::Item(err_schema) = &components.schemas["ErrorResponse"] else {
        panic!("ErrorResponse must be an inline component schema");
    };
    let SchemaKind::Type(Type::Object(err_obj)) = &err_schema.schema_kind else {
        panic!("ErrorResponse must be an object schema");
    };
    assert_eq!(err_obj.required, vec!["code", "message"]);

    fn verify_refs(node: &Value, m: &SpecMock) {
        match node {
            Value::Object(map) => {
                if map.contains_key("$ref") {
                    let _ = m.resolve_ref(node);
                }
                for v in map.values() {
                    verify_refs(v, m);
                }
            }
            Value::Array(arr) => {
                for v in arr {
                    verify_refs(v, m);
                }
            }
            _ => {}
        }
    }
    verify_refs(&m.raw, m);

    let mut operation_ids = BTreeSet::new();
    let mut covered_reqs = BTreeSet::new();
    for path_item in m.raw["paths"].as_object().unwrap().values() {
        for method in ["get", "post", "put", "patch", "delete"] {
            let Some(op) = path_item.get(method) else {
                continue;
            };
            let op_id = op["operationId"].as_str().expect("missing operationId");
            assert!(operation_ids.insert(op_id.to_string()));
            for r in op["x-requirements"].as_array().expect("missing x-requirements") {
                covered_reqs.insert(r.as_str().unwrap().to_string());
            }
        }
    }

    assert_eq!(operation_ids.len(), 26);
    for n in 1..=19 {
        assert!(covered_reqs.contains(&format!("REQ-F-{n:03}")));
    }
    assert!(covered_reqs.contains("REQ-NF-001"));
    assert!(covered_reqs.contains("REQ-NF-002"));
    assert!(covered_reqs.contains("REQ-NF-003"));

    let ReferenceOr::Item(Schema {
        schema_kind: SchemaKind::Type(Type::Object(order_obj)),
        ..
    }) = &components.schemas["RepairOrder"]
    else {
        panic!("RepairOrder schema missing");
    };
    for field in [
        "used_parts_cents",
        "completed_labor_cents",
        "labor_discount_cents",
        "settlement_cents",
    ] {
        let ReferenceOr::Item(prop) = &order_obj.properties[field] else {
            panic!("missing {field}");
        };
        assert!(matches!(prop.schema_kind, SchemaKind::Type(Type::Integer(_))));
    }
}
