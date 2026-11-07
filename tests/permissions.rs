//! The role matrix in `fixops-domain` must equal `x-roles` in the contract for every operation
//! (REQ-F-002, REQ-NF-002, ADD-001, ADD-002), so neither can change without the other.

use std::collections::{BTreeMap, BTreeSet};

use fixops_domain::staff::{Action, Role};
use serde_json::Value;

/// `operationId` → roles listed in `x-roles`, for every operation that has them.
fn contract_roles() -> BTreeMap<String, BTreeSet<String>> {
    let text = std::fs::read_to_string("spec/openapi.yaml").expect("readable contract");
    let spec: Value = serde_yaml::from_str(&text).expect("valid YAML");
    let mut roles = BTreeMap::new();
    for item in spec["paths"].as_object().expect("paths").values() {
        for op in item.as_object().expect("path item").values() {
            let (Some(id), Some(allowed)) = (op["operationId"].as_str(), op["x-roles"].as_array())
            else {
                continue;
            };
            let allowed = allowed
                .iter()
                .map(|r| r.as_str().expect("role name").to_owned())
                .collect();
            roles.insert(id.to_owned(), allowed);
        }
    }
    roles
}

#[test]
fn every_protected_operation_has_an_action() {
    let contract: BTreeSet<_> = contract_roles().into_keys().collect();
    let actions: BTreeSet<_> = Action::ALL
        .iter()
        .map(|a| a.operation_id().to_owned())
        .collect();
    assert_eq!(actions, contract);
}

#[test]
fn role_matrix_matches_x_roles() {
    let contract = contract_roles();
    for action in Action::ALL {
        let allowed = &contract[action.operation_id()];
        for role in Role::ALL {
            assert_eq!(
                role.can(action),
                allowed.contains(role.as_str()),
                "{role} on {}",
                action.operation_id()
            );
        }
    }
}

#[test]
fn only_sign_in_is_public() {
    let text = std::fs::read_to_string("spec/openapi.yaml").expect("readable contract");
    let spec: Value = serde_yaml::from_str(&text).expect("valid YAML");
    let protected = contract_roles();
    for item in spec["paths"].as_object().expect("paths").values() {
        for op in item.as_object().expect("path item").values() {
            let Some(id) = op["operationId"].as_str() else {
                continue;
            };
            let public = op["security"].as_array().is_some_and(Vec::is_empty);
            assert_eq!(public, !protected.contains_key(id), "{id}");
            assert_eq!(public, id == "createSession", "{id}");
        }
    }
}
