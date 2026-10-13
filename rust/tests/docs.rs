//! Documentation that states what the code can check: DESIGN.md's table of tactics lists the tactics of
//! `taxman::tactics::Tactics`, in order, with their defaults and their grades.

use std::path::Path;
use taxman::tactics::{Tactics, UNPROVEN};

/// One row of the table of tactics: the name, the default and the status, as written (without backticks).
struct Row {
    name: String,
    default: String,
    status: String,
}

/// The rows of the table in DESIGN.md's section "5. Tactics".
fn tactic_rows() -> Vec<Row> {
    let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("DESIGN.md")).expect("DESIGN.md");
    let section = text.split("\n## 5. Tactics").nth(1).expect("the section 5. Tactics");
    let section = section.split("\n## ").next().expect("the section's end");
    section
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            let cells: Vec<&str> = line.split('|').skip(1).map(|cell| cell.trim()).collect();
            let plain = |cell: &str| cell.trim_matches('`').to_owned();
            Row { name: plain(cells[0]), default: plain(cells[1]), status: cells[2].to_owned() }
        })
        .collect()
}

#[test]
fn design_lists_the_tactics_of_the_code_in_order() {
    let rows = tactic_rows();
    let listed: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(
        listed,
        Tactics::names(),
        "DESIGN.md section 5 and tactics::Tactics must list the same tactics in order"
    );
}

#[test]
fn design_gives_the_defaults_of_the_code() {
    for (row, (name, value)) in tactic_rows().iter().zip(Tactics::default().settings()) {
        assert_eq!(row.default, value.to_string(), "the default of {name}");
    }
}

#[test]
fn design_grades_the_unproven_tactics_as_the_code_lists_them() {
    let graded: Vec<String> = tactic_rows()
        .into_iter()
        .filter(|row| row.status.starts_with("unproven") || row.status.starts_with("unsound"))
        .map(|row| row.name)
        .collect();
    let listed: Vec<String> = UNPROVEN.iter().map(ToString::to_string).collect();
    assert_eq!(graded, listed, "the tactics DESIGN.md calls unproven or unsound are tactics::UNPROVEN");
}
