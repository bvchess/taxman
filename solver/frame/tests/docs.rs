//! Documentation that states what the code can check:
//!
//! * DESIGN.md's table of tactics lists the tactics of `taxman::tactics::Tactics`, in order, with their defaults;
//! * every relative link and `#anchor` of the repository's markdown files (the README, `docs/`, `results/`, `solver/`
//!   with the lab notebook, and the READMEs of `heuristics/` and `moniot/`) resolves;
//! * the lab notebook keeps its register and index on one page, `solver/notebook/README.md`, and its entries in the
//!   folders beside it, one README.md each: every entry opens with its title and a link back, and the register or the
//!   index links to it; `solver/docs/STATUS.md` is short and dated;
//! * README's table of the fields of a result line has the keys the code writes, and its example line shows them;
//! * README's reading order is the module map of `src/lib.rs`, and `rust-version` is what the docs and the cloud
//!   setup say;
//! * the docs call `unzip`'s moves *mandatory*, never *forced* (which is the search's word);
//! * the guides and comments do not use the words the project has retired (hive, apiary, model-free, upstream, cap
//!   for the ceiling, ...) or the old names of the prover's directory and script;
//! * the reference docs rewritten to docs/WRITING.md (`REWRITTEN`) do not use the names that docs/GLOSSARY.md
//!   replaced (board size, the optimum, strong bound, checker, EXPERIMENTS, ...).
//!
//! All of it reads files and solves a few small games, so the whole file takes a fraction of a second.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use taxman::json::Json;
use taxman::known::KnownSources;
use taxman::solver::{solve_game, SolveOptions};
use taxman::tactics::Tactics;

/// The crate's directory (`solver/frame/`).
fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The repository's root, two directories above the crate (with no `..` in it, so that paths below it can be shown
/// relative to it).
fn repo_dir() -> PathBuf {
    crate_dir().join("../..").canonicalize().expect("the repository above the crate")
}

/// The lab notebook's directory: its front page, `README.md`, and a folder for each entry.
fn notebook_dir() -> PathBuf {
    repo_dir().join("solver/notebook")
}

/// DESIGN.md, in the docs of the solver.
fn design_md() -> PathBuf {
    repo_dir().join("solver/docs/DESIGN.md")
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

/// One row of the table of tactics: the name and the default, as written (without backticks).
struct Row {
    name: String,
    default: String,
}

/// The rows of the table in DESIGN.md's section "5. Tactics".
fn tactic_rows() -> Vec<Row> {
    let text = read(design_md());
    let section = text.split("\n## 5. Tactics").nth(1).expect("the section 5. Tactics");
    let section = section.split("\n## ").next().expect("the section's end");
    section
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            let cells: Vec<&str> = line.split('|').skip(1).map(|cell| cell.trim()).collect();
            let plain = |cell: &str| cell.trim_matches('`').to_owned();
            Row { name: plain(cells[0]), default: plain(cells[1]) }
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

// ---------------------------------------------------------------------------------------------------------
// Links

/// The markdown files of the repository's docs: the README, `docs/`, `results/` and `solver/` (not the build
/// directories), and the READMEs of `heuristics/` and `moniot/`.
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
    let root = repo_dir();
    let mut found = vec![root.join("README.md")];
    for dir in ["docs", "results", "solver"] {
        walk(&root.join(dir), &mut found);
    }
    found.extend(["heuristics/README.md", "moniot/README.md"].iter().map(|name| root.join(name)));
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
    // image (solver/tools/cloud/Dockerfile) copies just the solver's directory and the data it needs.
    if !repo_dir().join("README.md").exists() {
        eprintln!("links not checked: this is not a full checkout of the repository (no README.md at its top)");
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
                let at = format!("{}:{}: ({target})", file.strip_prefix(repo_dir()).unwrap_or(&file).display(), i + 1);
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
// The lab notebook and the status page

/// The second line of every entry of the lab notebook, after a blank one.
const BACK_LINK: &str = "Part of the [lab notebook](../README.md).";

/// The sections of the notebook's front page: the introduction's, and no entry.
const FRONT_SECTIONS: [&str; 4] =
    ["How this notebook is kept", "Terms changed (2026-10)", "Register of everything tried", "Index of rounds"];

/// The README.md of every entry of the lab notebook, which is the folder `solver/notebook/NAME/` beside the front page;
/// sorted by name.
fn entry_files() -> Vec<PathBuf> {
    let dir = notebook_dir();
    let mut folders: Vec<PathBuf> =
        std::fs::read_dir(&dir).expect("solver/notebook/").map(|e| e.unwrap().path()).collect();
    folders.retain(|path| path.is_dir() && !path.file_name().unwrap().to_string_lossy().starts_with('.'));
    folders.sort();
    folders
        .into_iter()
        .map(|folder| {
            let readme = folder.join("README.md");
            assert!(readme.is_file(), "{} has no README.md: an entry is a folder with one", folder.display());
            readme
        })
        .collect()
}

/// The name of the entry whose README.md is `file`: its folder's.
fn entry_name(file: &Path) -> String {
    file.parent().and_then(Path::file_name).expect("an entry's folder").to_string_lossy().into_owned()
}

#[test]
fn every_entry_of_the_notebook_has_a_title_and_a_back_link_and_is_listed() {
    // What the register and the index link to: `NAME/README.md`, with or without an anchor.
    let front = read(notebook_dir().join("README.md"));
    let mut listed = BTreeSet::new();
    for heading in ["## Register of everything tried", "## Index of rounds"] {
        for line in without_code(section(&front, heading)) {
            for target in link_targets(&line) {
                listed.insert(target.split('#').next().unwrap_or("").to_owned());
            }
        }
    }
    let files = entry_files();
    assert!(files.len() > 40, "the check found only {} entries: has the layout changed?", files.len());
    let mut problems = Vec::new();
    for file in files {
        let name = entry_name(&file);
        let text = read(&file);
        let top: Vec<&str> = text.lines().take(3).collect();
        match top.first() {
            Some(title) if title.starts_with("# ") => {
                let named = anchors(title).into_iter().next().unwrap_or_default();
                if named != name {
                    problems.push(format!("{name}: named for its title's anchor, it would be {named}"));
                }
            }
            _ => problems.push(format!("{name}: the first line is not a title (`# `)")),
        }
        if top.get(1) != Some(&"") || top.get(2) != Some(&BACK_LINK) {
            problems.push(format!("{name}: the third line is not {BACK_LINK:?}, after a blank line"));
        }
        if !listed.contains(&format!("{name}/README.md")) {
            problems
                .push(format!("{name}: neither the register nor the index of solver/notebook/README.md links to it"));
        }
    }
    assert!(
        problems.is_empty(),
        "entries of solver/notebook/ that break the notebook's rules:\n{}",
        problems.join("\n")
    );
}

#[test]
fn the_front_page_of_the_notebook_has_no_entries() {
    let front = read(notebook_dir().join("README.md"));
    let sections: Vec<String> =
        without_code(&front).into_iter().filter_map(|line| line.strip_prefix("## ").map(str::to_owned)).collect();
    assert_eq!(
        sections, FRONT_SECTIONS,
        "solver/notebook/README.md keeps the register and the index; an entry is a folder of solver/notebook/"
    );
}

/// Whether `line` is `Updated YYYY-MM-DD.`, a plausible date.
fn is_update_line(line: &str) -> bool {
    let Some(date) = line.strip_prefix("Updated ").and_then(|rest| rest.strip_suffix('.')) else { return false };
    let parts: Vec<&str> = date.split('-').collect();
    let [year, month, day] = parts.as_slice() else { return false };
    let number = |part: &str, digits: usize| part.len() == digits && part.bytes().all(|b| b.is_ascii_digit());
    number(year, 4)
        && number(month, 2)
        && number(day, 2)
        && matches!(month.parse::<u32>(), Ok(1..=12))
        && matches!(day.parse::<u32>(), Ok(1..=31))
}

#[test]
fn status_is_short_and_dated() {
    let status = read(repo_dir().join("solver/docs/STATUS.md"));
    let lines = status.lines().count();
    assert!(
        lines <= 150,
        "solver/docs/STATUS.md has {lines} lines; it stays at 150 or fewer (rewrite it, do not append)"
    );
    assert!(
        status.lines().take(5).any(is_update_line),
        "solver/docs/STATUS.md says `Updated YYYY-MM-DD.` among its first lines"
    );
    assert!(
        is_update_line("Updated 2026-10-16.")
            && !is_update_line("Updated 2026-13-16.")
            && !is_update_line("Updated 2026-10-16")
    );
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
    // On its own, the default searches 1000 in more than the 1,024 steps after which the engine checks a limit.
    let stopped = keys(1000, &SolveOptions { node_limit: Some(1), ..opts.clone() });
    assert!(stopped.contains("best_so_far"), "a game that a limit stopped has best_so_far: {stopped:?}");
    // A game taken from a known source (`--use-prev`) cites it, in a key that only such a line has.
    let known = SolveOptions { use_prev: true, known: KnownSources { boards: true, ..Default::default() }, ..opts };
    let cited = keys(85, &known);
    assert_eq!(cited.difference(&complete).cloned().collect::<Vec<_>>(), ["known"], "a known game has `known`");
    (complete, stopped.union(&cited).cloned().collect())
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
    assert!(part.contains("null on a `trivial`, `prime` or `known` line"), "the table says when `setup` is null");
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
    let setup = read(repo_dir().join("solver/tools/cloud/setup.sh"));
    let min_rust = setup.lines().find_map(|l| l.strip_prefix("MIN_RUST=")).expect("MIN_RUST in setup.sh");
    assert_eq!(min_rust.split('#').next().unwrap().trim().trim_matches('"'), version, "setup.sh and Cargo.toml");
    let readme = read(crate_dir().join("README.md"));
    assert!(readme.contains(&format!("at least {version}")), "README names the toolchain version {version}");
}

// ---------------------------------------------------------------------------------------------------------
// Vocabulary

/// The text files that explain the code: the guides, the lab notebook with its entries, and the comments.
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
    let mut files: Vec<PathBuf> = vec![
        crate_dir().join("README.md"),
        design_md(),
        notebook_dir().join("README.md"),
        repo_dir().join("solver/docs/STATUS.md"),
    ];
    files.extend(entry_files());
    rust_files(&crate_dir().join("src"), &mut files);
    files
}

#[test]
fn unzips_moves_are_mandatory_not_forced() {
    // The search has forced members; a frame's move that `unzip` settles is *mandatory*.
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
    assert!(found.is_empty(), "say \"mandatory move\" for unzip's moves (docs/GLOSSARY.md):\n{}", found.join("\n"));
}

/// A word or phrase that the guides and comments no longer use, and what they say instead.
struct Retired {
    /// In lower case, matched as a whole word or phrase, not inside a longer word.  A plural is its own entry.
    word: &'static str,
    instead: &'static str,
}

const fn retired(word: &'static str, instead: &'static str) -> Retired {
    Retired { word, instead }
}

/// Retired in every guide and comment of the repository.
const RETIRED_EVERYWHERE: &[Retired] = &[
    retired("hive", "frame"),
    retired("hives", "frames"),
    retired("apiary", "the board's frames"),
    retired("model-free", "the prover's (proofs, search), or: on the whole board"),
    retired("evidence crate", "the prover"),
    retired("evidence solver", "the prover"),
    retired("exact solver", "the frame solver"),
    retired("main solver", "the frame solver"),
    retired("chain relation", "the ceiling"),
    retired("chain game", "a game at its ceiling"),
    retired("prime relation", "the prime formula"),
];

/// Retired in the guides and comments of the solver (`solver/` but for the prover), which use the wiki's
/// words.  The heuristics and the prover keep some of these for their own meanings (THEORY's frontier argument).
const RETIRED_IN_THE_SOLVER: &[Retired] = &[
    retired("upstream", "behind"),
    retired("downstream", "in front of"),
    retired("free factor", "freedoms"),
    retired("free factors", "freedoms"),
    retired("free hive", "a frame with freedoms"),
    retired("feasible promotion", "promotable"),
    retired("feasible completion", "promotable"),
    retired("feasible set", "promotable"),
    retired("feasible optimum", "promotable"),
    retired("cap", "ceiling (and `max_total`: the most promotions the freedoms allow)"),
    retired("caps", "ceilings"),
    retired("frontier", "the end of the chain, or the hardest games"),
    retired("strict run", "the committed chain"),
    retired("strict runs", "the committed chain"),
    retired("strict chain", "the committed chain"),
    retired("rust solver", "the frame solver"),
    retired("forced pair", "(a move left with one live factor is paid by it; no name)"),
    retired("forced pairs", "(a move left with one live factor is paid by it; no name)"),
    retired("many cores", "many CPUs or threads"),
    retired("few cores", "few CPUs or threads"),
    retired("all cores", "all CPUs"),
    retired("all the cores", "all the CPUs"),
    retired("cpu cores", "CPUs"),
    retired("one-level", "one frame forward"),
];

/// Retired in the README, DESIGN.md and the code's comments, which name the numbers by their part in the frame model,
/// and in the reference docs rewritten to docs/WRITING.md (`REWRITTEN`); the heuristics keep these words for the sets
/// they study.
const RETIRED_IN_THE_GUIDE_AND_CODE: &[Retired] = &[retired("upper half", "sources"), retired("lower half", "factors")];

/// The reference docs rewritten to docs/WRITING.md, by their paths from the repository.  The vocabulary of
/// docs/GLOSSARY.md (`RETIRED_IN_THE_REWRITTEN_DOCS`, `RETIRED_CASED`) applies to them; a doc joins the list when it is
/// rewritten.
const REWRITTEN: &[&str] = &["docs/GLOSSARY.md", "docs/WRITING.md"];

/// Retired in the rewritten docs: the names that docs/GLOSSARY.md replaced (2026-10-18).
const RETIRED_IN_THE_REWRITTEN_DOCS: &[Retired] = &[
    retired("board size", "game n, or n"),
    retired("board sizes", "the games, or the values of n"),
    retired("the optimum", "opt(n), the best score, or P*, the best promotion sum"),
    retired("the optima", "the values of opt(n)"),
    retired("strong bound", "the Lagrangian bound over the nogoods (the GF(2) bound inside, refitted at the node)"),
    retired("strong bounds", "the Lagrangian bound over the nogoods, in its forms"),
    retired("nogood bound", "the Lagrangian bound over the nogoods"),
    retired("nogood bounds", "the Lagrangian bound over the nogoods, in its forms"),
    retired("a1 bound", "the multi-field bound (A1)"),
    retired("a1 bounds", "the multi-field bound (A1)"),
    retired("fields bound", "the multi-field bound (A1)"),
    retired("checker", "the referee (one game) or a verifier (a whole results or bounds file)"),
    retired("checkers", "the verifiers"),
];

/// Retired in the rewritten docs and matched in their own case: the notebook's old name, which in lower case is a plain
/// English word.
const RETIRED_CASED: &[Retired] = &[retired("EXPERIMENTS", "the notebook (solver/notebook/README.md)")];

/// Paths and names that must not appear at all, not even in code: they were renamed.
const RETIRED_NAMES: &[Retired] = &[
    retired("rust/evidence", "solver/noframe"),
    retired("../evidence", "../noframe"),
    retired("evidence.sh", "prover.sh"),
    retired("evidence_only", "PROVER_ONLY"),
    retired("taxman_evidence", "taxman_prover"),
    retired("taxman-evidence", "taxman-prover"),
    retired("src/hives.rs", "src/frames/"),
    retired("golden_hives", "golden_frames"),
    retired("framecolumn", "SearchFrames (the search's frames are model::Frame)"),
    retired("free_factor_count", "freedoms"),
    retired("has_free_factors", "is_closed"),
];

/// Phrases that contain a retired word and stay: the heuristics' and the matroids' own meanings, and plain English
/// that is not about the frame model, and the name of the entry "The frontier chain to 3440", which the guides use when
/// they point to it.
const STAYS: &[&str] = &[
    "frontier argument",
    "frontier jam",
    "frontier chain",
    "odd core",
    "odd cores",
    "memory cap",
    "mem-cap",
    "cap reached",
];

/// The file the check skips, by its path from the repository: this one, which lists the words.
const SKIPPED: &str = "solver/frame/tests/docs.rs";

/// Whether the file at `shown` (a path from the repository) is an entry of the lab notebook, `solver/notebook/NAME/
/// README.md`: the entries are history and keep the words of their time, so the check skips them.
fn is_entry(shown: &str) -> bool {
    let parts: Vec<&str> = shown.split('/').collect();
    matches!(parts.as_slice(), ["solver", "notebook", _, "README.md"])
}

/// `line` with the targets of its links (`](target)`) taken out: a target names a file or an anchor, not a word.
fn without_link_targets(line: &str) -> String {
    let (mut out, mut rest) = (String::new(), line);
    while let Some(start) = rest.find("](") {
        out.push_str(&rest[..start + 1]);
        let after = &rest[start + 2..];
        let Some(end) = after.find(')') else { return out };
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Whether the guide or code at `shown` (a path from the repository) names the numbers of the board by their part in
/// the frame model.
fn names_numbers_by_frame(shown: &str) -> bool {
    shown.starts_with("solver/frame/src/")
        || shown == "solver/frame/README.md"
        || shown == "solver/docs/DESIGN.md"
        || REWRITTEN.contains(&shown)
}

/// Whether `word` occurs in `line` (lower case) as a whole word that is not quoted (`"forced moves" in its docs`).
fn has_word(line: &str, word: &str) -> bool {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    line.match_indices(word).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + word.len()..].chars().next();
        !before.is_some_and(is_word) && !after.is_some_and(is_word) && before != Some('"')
    })
}

/// The retired words in `lines` (prose: code blanked): `(line number, word, instead)`.
fn retired_in(lines: &[String], lists: &[&[Retired]]) -> Vec<(usize, &'static str, &'static str)> {
    let mut found = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let mut lower = without_link_targets(line).to_lowercase();
        for stays in STAYS {
            lower = lower.replace(stays, &" ".repeat(stays.len()));
        }
        for entry in lists.iter().flat_map(|list| list.iter()) {
            if has_word(&lower, entry.word) {
                found.push((i + 1, entry.word, entry.instead));
            }
        }
    }
    found
}

/// The retired words of `list` in `lines`, matched in their own case (`RETIRED_CASED`): `(line number, word, instead)`.
fn retired_cased_in(lines: &[String], list: &[Retired]) -> Vec<(usize, &'static str, &'static str)> {
    let mut found = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let line = without_link_targets(line);
        for entry in list.iter().filter(|entry| has_word(&line, entry.word)) {
            found.push((i + 1, entry.word, entry.instead));
        }
    }
    found
}

/// The front page of the lab notebook, `solver/notebook/README.md`, describes the present: all of it, but for the note
/// that maps the old words, which keeps them, and the titles of the entries in the first column of the index, which
/// keep the words of their time.  Those are blanked or cut, so that line numbers stay.  The entries themselves
/// (`solver/notebook/NAME/README.md`) are history and are skipped (see `is_entry`).
fn experiments_now(text: &str) -> String {
    let (before, after) = text.split_once("## Terms changed (2026-10)").expect("the note on the terms changed");
    let (note, register) = after.split_once("\n## Register of everything tried").expect("the register");
    let blank = |part: &str| vec![""; part.lines().count()].join("\n");
    let titles_out = |line: &str| match line.starts_with("| [") {
        true => line.splitn(3, '|').nth(2).unwrap_or("").to_owned(),
        false => line.to_owned(),
    };
    let register: Vec<String> = register.lines().map(titles_out).collect();
    format!("{before}{}\n## Register of everything tried\n{}", blank(note), register.join("\n"))
}

/// The prose of a file as lines: a markdown file's text, a Rust file's comments, a shell script's comments, with code
/// spans and fenced blocks blanked.  Lines are kept, so that line numbers still mean something.
fn prose_lines(path: &Path, text: &str) -> Vec<String> {
    let extension = path.extension().and_then(|ext| ext.to_str());
    if extension == Some("md") {
        return without_code(text);
    }
    let comment = |line: &str| -> String {
        let trimmed = line.trim_start();
        let after = if extension == Some("rs") {
            match trimmed.starts_with("//") {
                true => Some(trimmed.trim_start_matches('/').trim_start_matches('!')),
                false => line.split_once(" // ").map(|(_, comment)| comment),
            }
        } else {
            trimmed.strip_prefix('#').filter(|_| !trimmed.starts_with("#!"))
        };
        after.unwrap_or("").to_owned()
    };
    let comments: Vec<String> = text.lines().map(comment).collect();
    without_code(&comments.join("\n"))
}

/// `path` as shown in a message and compared with the names above: relative to the repository, with `/`.
fn from_repo(path: &Path) -> String {
    path.strip_prefix(repo_dir()).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

/// Whether the file at `shown` is one where the solver's extra retired words do not apply: the prover's, and the
/// repository's own (the README, `docs/`, `heuristics/`, `moniot/`).  The solver's are `solver/` but for the prover,
/// and the README of `results/`.
fn keeps_own_words(shown: &str) -> bool {
    shown.starts_with("solver/noframe/") || !(shown.starts_with("solver/") || shown.starts_with("results/"))
}

/// The files to check: the solver's directory and `results/` (which a Linux dry run has), and, in a full checkout, the
/// repository's README and `docs/`, and the READMEs of `heuristics/` and `moniot/`.
fn vocabulary_files() -> Vec<PathBuf> {
    fn walk(dir: &Path, found: &mut Vec<PathBuf>) {
        let mut paths: Vec<PathBuf> = std::fs::read_dir(dir).expect("a directory").map(|e| e.unwrap().path()).collect();
        paths.sort();
        for path in paths {
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            if path.is_dir() {
                if !matches!(name.as_str(), "target" | "testdata" | "data") && !name.starts_with('.') {
                    walk(&path, found);
                }
            } else if name.ends_with("_output.md") {
                // written by a study's script and committed as it was
            } else if matches!(path.extension().and_then(|e| e.to_str()), Some("md" | "rs" | "sh")) {
                found.push(path);
            }
        }
    }
    let root = repo_dir();
    let mut found = Vec::new();
    walk(&root.join("solver"), &mut found);
    walk(&root.join("results"), &mut found);
    if root.join("README.md").exists() {
        walk(&root.join("docs"), &mut found);
        found.extend(["README.md", "heuristics/README.md", "moniot/README.md"].iter().map(|name| root.join(name)));
    }
    found.into_iter().filter(|path| from_repo(path) != SKIPPED && !is_entry(&from_repo(path))).collect()
}

#[test]
fn the_retired_words_stay_out_of_the_guides_and_comments() {
    let mut found = Vec::new();
    let files = vocabulary_files();
    assert!(files.len() > 40, "the check found only {} files: has the layout changed?", files.len());
    for path in files {
        let shown = from_repo(&path);
        let mut text = read(&path);
        if shown == "solver/notebook/README.md" {
            text = experiments_now(&text);
        }
        let mut lists: Vec<&[Retired]> = vec![RETIRED_EVERYWHERE];
        if !keeps_own_words(&shown) {
            lists.push(RETIRED_IN_THE_SOLVER);
        }
        if names_numbers_by_frame(&shown) {
            lists.push(RETIRED_IN_THE_GUIDE_AND_CODE);
        }
        let prose = prose_lines(&path, &text);
        let mut retired = retired_in(&prose, &lists);
        if REWRITTEN.contains(&shown.as_str()) {
            retired.extend(retired_in(&prose, &[RETIRED_IN_THE_REWRITTEN_DOCS]));
            retired.extend(retired_cased_in(&prose, RETIRED_CASED));
        }
        for (at, word, instead) in retired {
            found.push(format!("{shown}:{at}: {word:?}, say {instead}"));
        }
        // The renamed paths and names are looked for as they are written, code included.
        for (i, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            for name in RETIRED_NAMES.iter().filter(|name| lower.contains(name.word)) {
                found.push(format!("{shown}:{}: {:?}, now {}", i + 1, name.word, name.instead));
            }
        }
    }
    assert!(
        found.is_empty(),
        "words and names the project has retired (solver/notebook/README.md, \"Terms changed\"):\n{}",
        found.join("\n")
    );
}

#[test]
fn the_vocabulary_check_finds_what_it_should() {
    let text = "A hive is a frame.\nthe `Hive` type, the apiary_x module and \"hives\" in the Java\n\
                see [a hive](hives.md) and [downstream](x.md#downstream)\n\
                a memory cap, the frontier argument, odd cores, the cap and the many Cores";
    let prose = without_code(text);
    let found = retired_in(&prose, &[RETIRED_EVERYWHERE, RETIRED_IN_THE_SOLVER]);
    let found: Vec<(usize, &str)> = found.iter().map(|&(at, word, _)| (at, word)).collect();
    assert_eq!(found, [(1, "hive"), (3, "hive"), (3, "downstream"), (4, "cap"), (4, "many cores")]);
    assert_eq!(without_link_targets("[a](b.md#c) and [d](e)"), "[a] and [d]");

    let text = "every Board Size, the optimum's score, the `board size` and the \"strong bound\" (quoted)\n\
                the experiments of EXPERIMENTS and [the notebook](EXPERIMENTS.md), and a type checker";
    let prose = without_code(text);
    let mut found = retired_in(&prose, &[RETIRED_IN_THE_REWRITTEN_DOCS]);
    found.extend(retired_cased_in(&prose, RETIRED_CASED));
    let found: Vec<(usize, &str)> = found.iter().map(|&(at, word, _)| (at, word)).collect();
    assert_eq!(found, [(1, "board size"), (1, "the optimum"), (2, "checker"), (2, "EXPERIMENTS")]);
}

/// The value of the constant `name` at the top of `solver/tools/quick_check.py`.
fn quick_check_limit(name: &str) -> f64 {
    let script = read(repo_dir().join("solver/tools/quick_check.py"));
    let line = script.lines().find_map(|line| line.strip_prefix(&format!("{name} = "))).expect("a constant");
    line.split_whitespace().next().and_then(|value| value.parse().ok()).expect("a number")
}

/// The quick check (`solver/tools/quick_check.py`) passes its own tests, and DESIGN.md states its limits as the script
/// has them, so that the number in the one is the number in the other.
#[test]
fn the_quick_check_passes_its_own_tests_and_design_states_its_limits() {
    let tests = repo_dir().join("solver/tools/test_quick_check.py");
    let run = std::process::Command::new("python3").arg("-I").arg(tests).output().expect("python3 runs the tests");
    assert!(run.status.success(), "{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));

    let design = read(design_md()).replace('\n', " ");
    let (target, limit, cpu) =
        (quick_check_limit("TOTAL_TARGET"), quick_check_limit("TOTAL_LIMIT"), quick_check_limit("TEST_LIMIT"));
    for stated in [format!("more than {limit} s (the target is {target} s;"), format!("more than {cpu} s of CPU")] {
        assert!(design.contains(&stated), "DESIGN.md, section 9, does not say {stated:?}: the script's limits changed");
    }
}
