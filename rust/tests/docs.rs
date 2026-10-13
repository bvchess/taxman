//! Documentation that states what the code can check:
//!
//! * DESIGN.md's table of tactics lists the tactics of `taxman::tactics::Tactics`, in order, with their defaults and
//!   their grades;
//! * every relative link and `#anchor` of the markdown files under `rust/` and of the repository's README resolves;
//! * README's table of the fields of a result line has the keys the code writes, and its example line shows them;
//! * README's reading order is the module map of `src/lib.rs`, and `rust-version` is what the docs and the cloud
//!   setup say;
//! * the docs call `unzip`'s moves *mandatory*, never *forced* (which is the search's word).
//!
//! All of it reads files and solves a few small games, so the whole file takes a fraction of a second.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use taxman::json::Json;
use taxman::solver::{solve_game, SolveOptions};
use taxman::tactics::{Tactics, UNPROVEN};

/// The crate's directory (`rust/`).
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The text of the part of `text` that starts after `heading` (a line start, `#`s and all) and ends before the next
/// heading of that level or a higher one.
fn section<'a>(text: &'a str, heading: &str) -> &'a str {
    let level = heading.split(' ').next().expect("a heading").len();
    let after = text.split(&format!("\n{heading}")).nth(1).unwrap_or_else(|| panic!("no section {heading:?}"));
    let after = after.split_once('\n').map_or("", |(_, rest)| rest);
    let ends = (1..=level).map(|n| format!("\n{} ", "#".repeat(n)));
    let end = ends.filter_map(|marker| after.find(&marker)).min().unwrap_or(after.len());
    &after[..end]
}

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

// ---------------------------------------------------------------------------------------------------------
// Links

/// The markdown files of the crate (not its build directory), and the repository's README.
fn markdown_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir).expect("a directory").map(|e| e.unwrap().path()).collect();
        paths.sort();
        for path in paths {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if path.is_dir() {
                if name != "target" && !name.starts_with('.') {
                    walk(&path, found);
                }
            } else if name.ends_with(".md") {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    walk(&crate_dir(), &mut found);
    found.push(crate_dir().join("../README.md"));
    found
}

/// `text` with its fenced code blocks and inline code spans blanked out, line by line (lines are kept, so that
/// line numbers still mean something).
fn without_code(text: &str) -> Vec<String> {
    let mut fenced = false;
    text.lines()
        .map(|line| {
            if line.trim_start().starts_with("```") {
                fenced = !fenced;
                return String::new();
            }
            if fenced {
                return String::new();
            }
            let mut in_span = false;
            line.chars()
                .filter(|&c| {
                    if c == '`' {
                        in_span = !in_span;
                    }
                    !in_span && c != '`'
                })
                .collect()
        })
        .collect()
}

/// The anchors GitHub gives the headings of a markdown file: the heading in lower case, punctuation but `-` and `_`
/// dropped, spaces turned into `-`, and `-1`, `-2` ... added to a heading that repeats.
fn anchors(text: &str) -> BTreeSet<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut anchors = BTreeSet::new();
    let fenced_out = text.lines().scan(false, |fenced, line| {
        if line.trim_start().starts_with("```") {
            *fenced = !*fenced;
        }
        Some(if *fenced { "" } else { line })
    });
    for line in fenced_out {
        let Some(heading) = line.strip_prefix('#').map(|rest| rest.trim_start_matches('#')) else { continue };
        let Some(heading) = heading.strip_prefix(' ') else { continue };
        let slug: String = heading
            .trim()
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' '))
            .map(|c| if c == ' ' { '-' } else { c })
            .collect();
        let count = seen.entry(slug.clone()).or_insert(0);
        anchors.insert(if *count == 0 { slug } else { format!("{slug}-{count}") });
        *count += 1;
    }
    anchors
}

/// The targets of the inline links `[text](target)` of a line (with code already blanked out).
fn link_targets(line: &str) -> Vec<&str> {
    let mut targets = Vec::new();
    let mut rest = line;
    while let Some(start) = rest.find("](") {
        let after = &rest[start + 2..];
        let Some(end) = after.find(')') else { break };
        targets.push(&after[..end]);
        rest = &after[end..];
    }
    targets
}

#[test]
fn every_relative_link_and_anchor_in_the_markdown_files_resolves() {
    // The links reach across the whole repository, so they can only be checked in a full checkout; the Linux dry-run
    // image (tools/cloud/Dockerfile) copies just the crate and the data it needs.
    if !crate_dir().join("../README.md").exists() {
        eprintln!("links not checked: this is not a full checkout of the repository (no ../README.md)");
        return;
    }
    let mut anchors_of: BTreeMap<PathBuf, BTreeSet<String>> = BTreeMap::new();
    let mut broken = Vec::new();
    let mut checked = 0;
    for file in markdown_files() {
        let text = read(&file);
        for (i, line) in without_code(&text).iter().enumerate() {
            for target in link_targets(line) {
                if target.contains("://") || target.starts_with("mailto:") {
                    continue;
                }
                checked += 1;
                let (path, fragment) = target.split_once('#').unwrap_or((target, ""));
                let linked = if path.is_empty() { file.clone() } else { file.parent().unwrap().join(path) };
                let at = format!("{}:{}: ({target})", file.strip_prefix(crate_dir()).unwrap_or(&file).display(), i + 1);
                if !linked.exists() {
                    broken.push(format!("{at}: there is no {}", linked.display()));
                } else if !fragment.is_empty() && linked.extension().is_some_and(|ext| ext == "md") {
                    let known = anchors_of.entry(linked.clone()).or_insert_with(|| anchors(&read(&linked)));
                    if !known.contains(fragment) {
                        broken.push(format!("{at}: no heading of {} has that anchor", linked.display()));
                    }
                }
            }
        }
    }
    assert!(broken.is_empty(), "broken links:\n{}", broken.join("\n"));
    assert!(checked > 50, "the checker found only {checked} relative links: has the link syntax changed?");
}

#[test]
fn anchors_follow_githubs_rules() {
    let text =
        "# Title\n## 7. Per-game pipeline (`solver`)\n```\n# not a heading\n```\n## Tactics\n## Tactics\n### A & B\n";
    let found: Vec<String> = anchors(text).into_iter().collect();
    assert_eq!(found, ["7-per-game-pipeline-solver", "a--b", "tactics", "tactics-1", "title"]);
    let lines = without_code("see [a](b.md#c) and `[x](y)`\n```\n[z](w)\n```\n[d](e)");
    let targets: Vec<&str> = lines.iter().flat_map(|line| link_targets(line)).collect();
    assert_eq!(targets, ["b.md#c", "e"]);
}

// ---------------------------------------------------------------------------------------------------------
// The README against the code

/// The backticked words in the first cell of each row of the markdown table in `text`.
fn first_column_words(text: &str) -> BTreeSet<String> {
    let rows = text.lines().filter(|line| line.starts_with("| `"));
    rows.flat_map(|row| {
        let cell = row.split('|').nth(1).unwrap_or("");
        cell.split('`').skip(1).step_by(2).map(str::to_owned).collect::<Vec<_>>()
    })
    .collect()
}

/// The keys of the top-level object in `text` (an example line with comments and `...` in it, so not valid JSON):
/// a string that is followed by a colon at depth 1 of the brackets.
fn top_level_keys(text: &str) -> BTreeSet<String> {
    let (mut keys, mut depth, mut chars) = (BTreeSet::new(), 0, text.chars().peekable());
    while let Some(c) = chars.next() {
        match c {
            '{' | '[' => depth += 1,
            '}' | ']' => depth -= 1,
            '#' => chars.by_ref().take_while(|&c| c != '\n').for_each(drop),
            '"' => {
                let word: String = chars.by_ref().take_while(|&c| c != '"').collect();
                if depth == 1 && chars.peek() == Some(&':') {
                    keys.insert(word);
                }
            }
            _ => {}
        }
    }
    keys
}

/// The keys of the result lines the code writes: those of a game that is searched and one that is not, and of one
/// that a limit stopped (which has `best_so_far`).
fn keys_written() -> (BTreeSet<String>, BTreeSet<String>) {
    let keys = |n: u32, opts: &SolveOptions| -> BTreeSet<String> {
        let line = solve_game(n, None, opts).expect("a game").to_json().to_string();
        let json = Json::parse(&line).expect("a result line is JSON");
        json.as_obj().expect("an object").iter().map(|(key, _)| key.clone()).collect()
    };
    let opts = SolveOptions { threads: 1, ..SolveOptions::default() };
    let complete = keys(85, &opts);
    assert_eq!(keys(12, &opts), complete, "a game with no search has the keys of one with a search");
    let stopped = keys(600, &SolveOptions { node_limit: Some(1), ..opts });
    assert!(stopped.contains("best_so_far"), "a game that a limit stopped has best_so_far: {stopped:?}");
    (complete, stopped)
}

#[test]
fn readme_documents_exactly_the_keys_of_a_result_line() {
    let readme = read(crate_dir().join("README.md"));
    let part = section(&readme, "### A results line");
    let (complete, stopped) = keys_written();

    let documented = first_column_words(part);
    let written: BTreeSet<String> = complete.union(&stopped).cloned().collect();
    assert_eq!(documented, written, "README's table of the fields of a result line, and the keys the code writes");

    let example = part.split("```").nth(1).expect("the example line is the first code block");
    assert_eq!(top_level_keys(example), complete, "README's example line, and the keys of a finished game");
    assert!(example.contains("\"setup\":{"), "the example is a searched game");
    assert!(part.contains("null on a `trivial`, `prime` or `reuse` line"), "the table says when `setup` is null");
}

// ---------------------------------------------------------------------------------------------------------
// Reading order, versions

/// The module names in order of appearance: of the rows of the module map in `src/lib.rs` (`[`name`]`, second
/// column) that are modules of the library.
fn lib_module_map() -> Vec<String> {
    let lib = read(crate_dir().join("src/lib.rs"));
    let declared: BTreeSet<&str> = lib.lines().filter_map(|l| l.strip_prefix("pub mod ")?.strip_suffix(';')).collect();
    let rows = lib.lines().filter(|line| line.starts_with("//! |"));
    let names = rows.filter_map(|row| row.split('|').nth(2)?.trim().strip_prefix("[`")?.split('`').next());
    let names: Vec<String> = names.filter(|name| declared.contains(name)).map(str::to_owned).collect();
    assert_eq!(names.iter().collect::<BTreeSet<_>>().len(), declared.len(), "every module is in the module map");
    names
}

#[test]
fn the_reading_order_of_the_readme_is_the_module_map_of_lib_rs() {
    let readme = read(crate_dir().join("README.md"));
    let reading = section(&readme, "## 8. Reading order and map of the documents");
    let list = reading.split("\n| document").next().unwrap();
    let declared = lib_module_map();
    let mut in_readme: Vec<String> = Vec::new();
    for word in list.split('`').skip(1).step_by(2) {
        let Some(path) = word.strip_prefix("src/") else { continue };
        let name = path.trim_end_matches('/').trim_end_matches(".rs");
        if declared.iter().any(|d| d == name) && !in_readme.iter().any(|seen| seen == name) {
            in_readme.push(name.to_owned());
        }
    }
    assert_eq!(in_readme, declared, "README's reading order (section 8) and the module map in src/lib.rs");
}

#[test]
fn the_minimum_rust_version_is_the_same_everywhere() {
    let cargo = read(crate_dir().join("Cargo.toml"));
    let version = cargo.lines().find_map(|l| l.strip_prefix("rust-version = ")).expect("rust-version in Cargo.toml");
    let version = version.trim_matches('"');
    let setup = read(crate_dir().join("tools/cloud/setup.sh"));
    let min_rust = setup.lines().find_map(|l| l.strip_prefix("MIN_RUST=")).expect("MIN_RUST in setup.sh");
    assert_eq!(min_rust.split('#').next().unwrap().trim().trim_matches('"'), version, "setup.sh and Cargo.toml");
    let readme = read(crate_dir().join("README.md"));
    assert!(readme.contains(&format!("at least {version}")), "README names the toolchain version {version}");
}

// ---------------------------------------------------------------------------------------------------------
// Vocabulary

/// The text files that explain the code: the guides and the comments.
fn prose_files() -> Vec<PathBuf> {
    fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("a directory") {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_files(&path, found);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                found.push(path);
            }
        }
    }
    let mut files: Vec<PathBuf> =
        ["README.md", "DESIGN.md", "docs/EXPERIMENTS.md"].iter().map(|name| crate_dir().join(name)).collect();
    rust_files(&crate_dir().join("src"), &mut files);
    files
}

#[test]
fn unzips_moves_are_mandatory_not_forced() {
    // The search has forced members, and `reduce` forced pairs; a hive's move that `unzip` settles is *mandatory*.
    // The Java's word may be quoted: `"forced moves" in its docs`.
    let mut found = Vec::new();
    for file in prose_files() {
        for (i, line) in read(&file).lines().enumerate() {
            let lower = line.to_lowercase();
            let unquoted = lower.match_indices("forced move").any(|(at, _)| !lower[..at].ends_with('"'));
            if unquoted {
                found.push(format!("{}:{}: {}", file.display(), i + 1, line.trim()));
            }
        }
    }
    assert!(found.is_empty(), "say \"mandatory move\" for unzip's moves (README, glossary):\n{}", found.join("\n"));
}
