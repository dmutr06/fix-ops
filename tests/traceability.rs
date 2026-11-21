//! Audits `spec/traceability.md` against the SRS, the contract and the repository: every
//! requirement has one complete row, scenario links match the SRS, every operation and test file
//! is traced, and every linked file and doc section exists.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_json::Value;

const SRS: &str = "spec/srs.md";
const MATRIX: &str = "spec/traceability.md";

fn read(path: &str) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"))
}

/// Requirement IDs defined in the SRS tables, in order.
fn srs_requirements(srs: &str) -> Vec<String> {
    srs.lines()
        .filter_map(|line| line.strip_prefix("| REQ-"))
        .map(|rest| format!("REQ-{}", &rest[..rest.find(' ').unwrap()]))
        .collect()
}

/// `BDD-nnn → requirements` from the scenario headings, expanding ranges like `REQ-F-003`–`005`.
fn srs_scenarios(srs: &str) -> BTreeMap<String, BTreeSet<String>> {
    let mut scenarios = BTreeMap::new();
    for line in srs.lines().filter(|l| l.starts_with("**BDD-")) {
        let id = line[2..9].to_owned();
        let heading_refs = &line[line.rfind('(').unwrap()..];
        let mut linked = BTreeSet::new();
        let tokens: Vec<&str> = heading_refs.split('`').collect();
        for (i, token) in tokens.iter().enumerate() {
            if let Some(number) = token.strip_prefix("REQ-") {
                linked.insert((*token).to_owned());
                let is_range = tokens.get(i + 1) == Some(&"–");
                if let (true, Some(end)) = (is_range, tokens.get(i + 2)) {
                    let (kind, start) = number.rsplit_once('-').unwrap();
                    for n in start.parse::<u32>().unwrap()..=end.parse().unwrap() {
                        linked.insert(format!("REQ-{kind}-{n:03}"));
                    }
                }
            }
        }
        scenarios.insert(id, linked);
    }
    scenarios
}

/// Rows of the matrix table whose header starts with `header`, keyed by requirement.
fn table(matrix: &str, header: &str) -> BTreeMap<String, Vec<String>> {
    let mut rows = BTreeMap::new();
    let mut lines = matrix
        .lines()
        .skip_while(|l| !l.starts_with(header))
        .skip(2);
    while let Some(line) = lines.next().filter(|l| l.starts_with('|')) {
        let cells: Vec<String> = line
            .trim_matches('|')
            .split(" | ")
            .map(|c| c.trim().to_owned())
            .collect();
        let previous = rows.insert(cells[0].clone(), cells[1..].to_vec());
        assert!(previous.is_none(), "{} appears twice in {header}", cells[0]);
    }
    rows
}

fn backticked(cell: &str) -> impl Iterator<Item = &str> {
    cell.split('`').skip(1).step_by(2)
}

/// Repository path of a file named in the implementation table.
fn resolve(name: &str) -> String {
    for (prefix, dir) in [
        ("domain/", "crates/domain/src/"),
        ("app/", "crates/app/src/"),
    ] {
        if let Some(rest) = name.strip_prefix(prefix) {
            return format!("{dir}{rest}");
        }
    }
    if name.contains('/') {
        name.to_owned()
    } else {
        format!("tests/{name}")
    }
}

/// GitHub's anchor for a Markdown heading.
fn slug(heading: &str) -> String {
    heading
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .map(|c| if c == ' ' { '-' } else { c })
        .collect()
}

#[test]
fn every_requirement_has_one_row_in_each_table() {
    let required: BTreeSet<_> = srs_requirements(&read(SRS)).into_iter().collect();
    assert_eq!(required.len(), 24);
    let matrix = read(MATRIX);
    for header in ["| Requirement | AC |", "| Requirement | Tests |"] {
        let rows: BTreeSet<_> = table(&matrix, header).into_keys().collect();
        assert_eq!(rows, required, "rows of {header}");
    }
}

#[test]
fn acceptance_criteria_and_addenda_exist() {
    let srs = read(SRS);
    let addenda: Vec<String> = std::fs::read_dir("spec/addenda")
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    for (req, cells) in table(&read(MATRIX), "| Requirement | AC |") {
        assert_eq!(cells[0], req.replace("REQ-", "AC-"), "{req}: AC");
        assert!(
            srs.contains(&format!("{}:", cells[0])),
            "{req}: AC not in SRS"
        );
        assert!(
            !cells[3].is_empty() && !cells[4].is_empty(),
            "{req}: package or tickets empty"
        );
        for addendum in cells[5].split(", ").filter(|a| !a.is_empty()) {
            assert!(
                addenda.iter().any(|f| f.starts_with(addendum)),
                "{req}: unknown {addendum}"
            );
        }
    }
}

#[test]
fn scenario_links_match_the_srs() {
    let scenarios = srs_scenarios(&read(SRS));
    assert_eq!(scenarios.len(), 21);
    let mut expected: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (bdd, reqs) in &scenarios {
        for req in reqs {
            expected
                .entry(req.clone())
                .or_default()
                .insert(bdd[4..].to_owned());
        }
    }
    for (req, cells) in table(&read(MATRIX), "| Requirement | AC |") {
        let listed: BTreeSet<String> = cells[1]
            .split(", ")
            .filter(|b| *b != "—")
            .map(str::to_owned)
            .collect();
        let want = expected.remove(&req).unwrap_or_default();
        assert_eq!(
            listed, want,
            "{req}: BDD column differs from the SRS headings"
        );
    }
    assert!(
        expected.is_empty(),
        "scenarios name unknown requirements: {expected:?}"
    );
}

#[test]
fn every_contract_operation_is_traced() {
    let spec: Value = serde_yaml::from_str(&read("spec/openapi.yaml")).unwrap();
    let operations: BTreeSet<String> = spec["paths"]
        .as_object()
        .unwrap()
        .values()
        .flat_map(|item| item.as_object().unwrap().values())
        .filter_map(|op| op["operationId"].as_str().map(str::to_owned))
        .collect();
    let mut traced = BTreeSet::new();
    for (req, cells) in table(&read(MATRIX), "| Requirement | AC |") {
        for name in backticked(&cells[2]).filter(|n| !["bearerAuth", "x-roles"].contains(n)) {
            assert!(operations.contains(name), "{req}: unknown operation {name}");
            traced.insert(name.to_owned());
        }
    }
    assert_eq!(traced, operations, "operations without a requirement");
}

#[test]
fn linked_files_and_sections_exist() {
    for (req, cells) in table(&read(MATRIX), "| Requirement | Tests |") {
        for cell in &cells[..2] {
            assert!(backticked(cell).next().is_some(), "{req}: empty cell");
            for name in backticked(cell) {
                let path = resolve(name);
                assert!(Path::new(&path).is_file(), "{req}: {path} does not exist");
            }
        }
        assert!(cells[2].contains("]("), "{req}: no docs link");
        for link in cells[2].split("](../").skip(1) {
            let target = &link[..link.find(')').unwrap()];
            let (file, anchor) = target.split_once('#').unwrap_or((target, ""));
            let text = read(file);
            if !anchor.is_empty() {
                let found = text
                    .lines()
                    .filter_map(|l| l.strip_prefix('#'))
                    .any(|h| slug(h.trim_start_matches('#').trim()) == anchor);
                assert!(found, "{req}: {file} has no section #{anchor}");
            }
        }
    }
}

#[test]
fn every_test_file_is_traced() {
    let matrix = read(MATRIX);
    for entry in std::fs::read_dir("tests").unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        let is_test = Path::new(&name)
            .extension()
            .is_some_and(|ext| ext == "rs" || ext == "sh");
        if is_test && name != "recovery_cleanup.sh" {
            assert!(
                matrix.contains(&format!("`{name}`"))
                    || matrix.contains(&format!("`tests/{name}`")),
                "tests/{name} is not in the matrix"
            );
        }
    }
}
