//! Documentation that states what the code can check:
//!
//! * the table of tactics in the design (`solver/docs/design/tactics.md`) lists the tactics of
//!   `taxman::tactics::Tactics`, in order, with their defaults;
//! * every relative link and `#anchor` of the repository's markdown files (the README, `docs/`, `results/`, `solver/`
//!   with the lab notebook, and the READMEs of `heuristics/` and `moniot/`) resolves;
//! * the lab notebook keeps its rules on its front page, `solver/notebook/README.md`, its register in the files of
//!   `register/` (a file a subject), its index in `index.md`, and its entries in the folders beside them, one README.md
//!   each: every entry opens with its title and a link back, and the register or the index links to it, and the front
//!   page holds no entry and no table row; `solver/docs/STATUS.md` is short and dated;
//! * the reference's (`solver/docs/REFERENCE.md`) table of the fields of a result line has the keys the code writes,
//!   and its example line shows them;
//! * the design's reading order (`solver/docs/design/architecture.md`) is the module map of `src/lib.rs`, and
//!   `rust-version` is what the README and the cloud setup say;
//! * the docs call `unzip`'s moves *mandatory*, never *forced* (which is the search's word);
//! * the guides and comments do not use the words the project has retired (hive, apiary, model-free, upstream, cap
//!   for the ceiling, ...) or the old names of the prover's directory and script, but for the note that lists them,
//!   `solver/notebook/terms-changed.md`;
//! * the docs rewritten to docs/WRITING.md (`REWRITTEN`), and the comments of the code (the Rust files of both crates
//!   and the scripts of the tools: `CODE_DIRS`, `SCRIPT_DIRS`), do not use the names that docs/GLOSSARY.md replaced
//!   (board size, the optimum, strong bound, checker, EXPERIMENTS, ...);
//! * every heading that a guide, a comment or a script cites by name (`WRITING.md, "Give each task one file"`) or by
//!   number (`WRITING.md, section 8`) exists in the document it names, and so does every `PATH.md#anchor` cited in the
//!   code;
//! * every guide but the notebook's entries keeps the limit of words of docs/WRITING.md, rule 8, but those still to be
//!   split (`STILL_TO_SPLIT`), and the old files of the split documents (`INDEXES`) stay under the limit of an index.
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

/// The folder of the design's files, by its path from the repository, ending in `/` as the folders of `REWRITTEN` do:
/// a file for each part of the design (tactics.md, architecture.md, testing.md, ...), each opening with its title.
const DESIGN_DIR: &str = "solver/docs/design/";

/// DESIGN.md, in the docs of the solver: the index of the design (its goals and the table of its files).
fn design_md() -> PathBuf {
    repo_dir().join("solver/docs/DESIGN.md")
}

/// The folder of the design's files (`DESIGN_DIR`).
fn design_dir() -> PathBuf {
    repo_dir().join(DESIGN_DIR)
}

/// REFERENCE.md, the frame solver's reference, in the docs of the solver.
fn reference_md() -> PathBuf {
    repo_dir().join("solver/docs/REFERENCE.md")
}

fn read(path: impl AsRef<Path>) -> String {
    let path = path.as_ref();
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// The text of the design's file `name` (`tactics.md`, ...) after its title.  A file of the design is one whole part
/// of it, from its `# Title` on, so what follows the title is the part, whatever headings it has inside.
fn design_file(name: &str) -> String {
    let text = read(design_dir().join(name));
    let (title, body) = text.split_once('\n').unwrap_or((&text, ""));
    assert!(
        title.starts_with("# "),
        "{DESIGN_DIR}{name} must open with its title, a line `# Title`, as the files of the design do; \
         it opens with {title:?}"
    );
    body.to_owned()
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

/// The rows of the table in the design's `tactics.md`.
fn tactic_rows() -> Vec<Row> {
    design_file("tactics.md")
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
        "the table of {DESIGN_DIR}tactics.md and tactics::Tactics must list the same tactics in order"
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

/// The lines of `text` with those of fenced code blocks (and the fences) emptied (lines are kept, so that line numbers
/// still mean something).
fn outside_fences(text: &str) -> Vec<&str> {
    let mut fenced = false;
    text.lines()
        .map(|line| {
            let fence = line.trim_start().starts_with("```");
            fenced ^= fence;
            if fence || fenced {
                ""
            } else {
                line
            }
        })
        .collect()
}

/// `text` with its fenced code blocks and inline code spans blanked out, line by line (lines are kept, so that
/// line numbers still mean something).
fn without_code(text: &str) -> Vec<String> {
    outside_fences(text)
        .into_iter()
        .map(|line| {
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

/// The headings of a markdown file, at any level and outside fenced code: their text, trimmed, without the `#`s.
fn headings(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    for line in outside_fences(text) {
        let Some(rest) = line.strip_prefix('#') else { continue };
        if let Some(heading) = rest.trim_start_matches('#').strip_prefix(' ') {
            found.push(heading.trim());
        }
    }
    found
}

/// The anchors GitHub gives the headings of a markdown file: the heading in lower case, punctuation but `-` and `_`
/// dropped, spaces turned into `-`, and `-1`, `-2` ... added to a heading that repeats.
fn anchors(text: &str) -> BTreeSet<String> {
    let mut seen: BTreeMap<String, usize> = BTreeMap::new();
    let mut anchors = BTreeSet::new();
    for heading in headings(text) {
        let slug: String = heading
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

/// The byte ranges of the targets of the inline links `[text](target)` of `text` (with code already blanked out).
fn link_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut from = 0;
    while let Some(open) = text[from..].find("](") {
        let start = from + open + 2;
        let Some(length) = text[start..].find(')') else { break };
        ranges.push(start..start + length);
        from = start + length;
    }
    ranges
}

/// The targets of the inline links `[text](target)` of a line (with code already blanked out).
fn link_targets(line: &str) -> Vec<&str> {
    link_ranges(line).into_iter().map(|range| &line[range]).collect()
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

/// The sections of the notebook's front page: the rules for filing an entry and for the register, and no entry.
const FRONT_SECTIONS: [&str; 2] = ["How this notebook is kept", "Register of everything tried"];

/// The folder of the lab notebook that holds the register, a file a subject: it is not an entry.
const REGISTER_FOLDER: &str = "register";

/// The notebook's index of rounds, by its path from the repository.
const NOTEBOOK_INDEX: &str = "solver/notebook/index.md";

/// The notebook's note that maps the words the project retired to the words that replaced them, by its path from the
/// repository.  It lists the old words, so the vocabulary check skips it.
const TERMS_CHANGED: &str = "solver/notebook/terms-changed.md";

/// The README.md of every entry of the lab notebook, which is the folder `solver/notebook/NAME/` beside the front page
/// (the folder `register/` is not an entry); sorted by name.
fn entry_files() -> Vec<PathBuf> {
    let dir = notebook_dir();
    let mut folders: Vec<PathBuf> =
        std::fs::read_dir(&dir).expect("solver/notebook/").map(|e| e.unwrap().path()).collect();
    folders.retain(|path| {
        let name = path.file_name().unwrap().to_string_lossy();
        path.is_dir() && !name.starts_with('.') && name != REGISTER_FOLDER
    });
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

/// The folder of an entry of the lab notebook and the path below it, for the file at `shown` (a path from the
/// repository), if it lies in an entry's folder: `("NAME", "README.md")` for `solver/notebook/NAME/README.md`.  The
/// front page, `index.md`, `terms-changed.md` and the files of `register/` lie in no entry's folder.
fn in_entry_folder(shown: &str) -> Option<(&str, &str)> {
    let (folder, below) = shown.strip_prefix("solver/notebook/")?.split_once('/')?;
    (folder != REGISTER_FOLDER).then_some((folder, below))
}

/// The files of the register, `solver/notebook/register/*.md`, a file a subject; sorted by path.
fn register_files() -> Vec<PathBuf> {
    files_with_extensions(&notebook_dir().join(REGISTER_FOLDER), &["md"])
}

/// The files that the links of the register's files and of the index reach (without their anchors): each link is
/// resolved from the folder of the file that holds it, so that `../NAME/README.md` and `NAME/README.md` both reach
/// the entry NAME.  A link to a file that does not exist reaches nothing (the link test finds it).
fn files_linked_by_register_and_index() -> BTreeSet<PathBuf> {
    let mut files = register_files();
    assert!(
        !files.is_empty(),
        "the check found no file in solver/notebook/{REGISTER_FOLDER}/: has the layout changed?"
    );
    files.push(repo_dir().join(NOTEBOOK_INDEX));
    let mut linked = BTreeSet::new();
    for file in files {
        for line in without_code(&read(&file)) {
            for target in link_targets(&line) {
                let path = target.split('#').next().unwrap_or("");
                if let Ok(reached) = file.parent().expect("a file has a directory").join(path).canonicalize() {
                    linked.insert(reached);
                }
            }
        }
    }
    linked
}

/// Every entry opens with its title, named for its anchor, and a back link, and the register or the index links to it.
#[test]
fn every_entry_of_the_notebook_has_a_title_and_a_back_link_and_is_listed() {
    let listed = files_linked_by_register_and_index();
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
        if !listed.contains(&file) {
            problems.push(format!(
                "{name}: neither a file of solver/notebook/{REGISTER_FOLDER}/ nor {NOTEBOOK_INDEX} links to it"
            ));
        }
    }
    assert!(
        problems.is_empty(),
        "entries of solver/notebook/ that break the notebook's rules:\n{}",
        problems.join("\n")
    );
}

/// The front page keeps the rules of the notebook and of the register (`FRONT_SECTIONS`), and no entry; and it holds no
/// table row, since the rows are in the files of `register/` and in `index.md`.
#[test]
fn the_front_page_of_the_notebook_has_no_entries_and_no_table_rows() {
    let front = without_code(&read(notebook_dir().join("README.md")));
    let sections: Vec<String> = front.iter().filter_map(|line| line.strip_prefix("## ").map(str::to_owned)).collect();
    assert_eq!(
        sections, FRONT_SECTIONS,
        "solver/notebook/README.md keeps the rules of the notebook and of the register; an entry is a folder of \
         solver/notebook/"
    );
    let rows: Vec<String> = front
        .iter()
        .enumerate()
        .filter(|(_, line)| line.starts_with("| "))
        .map(|(i, line)| format!("solver/notebook/README.md:{}: {line}", i + 1))
        .collect();
    assert!(
        rows.is_empty(),
        "the front page of the notebook holds no table: the rows of the register are in the files of \
         solver/notebook/{REGISTER_FOLDER}/ and the rows of the index in {NOTEBOOK_INDEX}:\n{}",
        rows.join("\n")
    );
}

/// The register's files, the index and the note on the terms changed lie in no entry's folder; a file in the folder
/// of an entry, at any depth, does.
#[test]
fn the_register_and_the_top_files_are_told_from_the_entries() {
    let beside = [
        "solver/notebook/README.md",
        NOTEBOOK_INDEX,
        TERMS_CHANGED,
        "solver/notebook/register/node-bound.md",
        "solver/notebook/register/README.md",
        "solver/docs/DESIGN.md",
    ];
    for shown in beside {
        assert!(!is_entry(shown) && !is_in_an_entry(shown), "{shown} lies in no entry's folder");
    }
    assert!(is_entry("solver/notebook/x-2026-10/README.md") && is_in_an_entry("solver/notebook/x-2026-10/README.md"));
    assert!(!is_entry("solver/notebook/x-2026-10/fields_output.md"));
    assert!(is_in_an_entry("solver/notebook/x-2026-10/fields_output.md") && is_in_an_entry("solver/notebook/x/y/z.md"));
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
fn the_reference_documents_exactly_the_keys_of_a_result_line() {
    let reference = read(reference_md());
    let part = section(&reference, "### A results line");
    let (complete, stopped) = keys_written();

    let documented = first_column_words(part);
    let written: BTreeSet<String> = complete.union(&stopped).cloned().collect();
    assert_eq!(
        documented, written,
        "REFERENCE.md's table of the fields of a result line, and the keys the code writes"
    );

    let example = part.split("```").nth(1).expect("the example line is the first code block");
    assert_eq!(top_level_keys(example), complete, "REFERENCE.md's example line, and the keys of a finished game");
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
fn the_reading_order_of_design_is_the_module_map_of_lib_rs() {
    let architecture = design_file("architecture.md");
    let declared = lib_module_map();
    let mut in_design: Vec<String> = Vec::new();
    for word in architecture.split('`').skip(1).step_by(2) {
        let Some(path) = word.strip_prefix("src/") else { continue };
        let name = path.trim_end_matches('/').trim_end_matches(".rs");
        if declared.iter().any(|d| d == name) && !in_design.iter().any(|seen| seen == name) {
            in_design.push(name.to_owned());
        }
    }
    assert_eq!(
        in_design, declared,
        "the reading order of {DESIGN_DIR}architecture.md and the module map in src/lib.rs"
    );
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

/// The text files that explain the code: the guides (DESIGN.md with the files of the design), the lab notebook (its
/// front page, register, index and note on the terms changed, and its entries), and the comments.
fn prose_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = vec![
        crate_dir().join("README.md"),
        reference_md(),
        design_md(),
        notebook_dir().join("README.md"),
        repo_dir().join(NOTEBOOK_INDEX),
        repo_dir().join(TERMS_CHANGED),
        repo_dir().join("solver/docs/STATUS.md"),
    ];
    files.extend(files_with_extensions(&design_dir(), &["md"]));
    files.extend(register_files());
    files.extend(entry_files());
    files.extend(files_with_extensions(&crate_dir().join("src"), &["rs"]));
    files
}

/// The files below `dir` (not in a `target` or a hidden directory) that `keep` accepts, in the order of their paths.
fn files_where(dir: &Path, keep: &dyn Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::fs::read_dir(dir).expect("a directory").map(|e| e.unwrap().path()).collect();
    paths.sort();
    let mut found = Vec::new();
    for path in paths {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        if path.is_dir() {
            if name != "target" && !name.starts_with('.') {
                found.extend(files_where(&path, keep));
            }
        } else if keep(&path) {
            found.push(path);
        }
    }
    found
}

/// The files below `dir` (not in a `target` or a hidden directory) whose extension is one of `extensions`, in the
/// order of their paths.
fn files_with_extensions(dir: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    files_where(dir, &|path| path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| extensions.contains(&ext)))
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

/// Retired in the README, DESIGN.md with the files of the design, and the code's comments, which name the numbers by
/// their part in the frame model, and in the docs rewritten to docs/WRITING.md (`REWRITTEN`) but those that speak of
/// the whole game (`SPEAK_OF_THE_WHOLE_GAME`).
const RETIRED_IN_THE_GUIDE_AND_CODE: &[Retired] = &[retired("upper half", "sources"), retired("lower half", "factors")];

/// The docs rewritten to docs/WRITING.md, by their paths from the repository.  The vocabulary of docs/GLOSSARY.md
/// (`RETIRED_IN_THE_REWRITTEN_DOCS`, `RETIRED_CASED`) applies to them; a doc joins the list when it is rewritten.
const REWRITTEN: &[&str] = &[
    "docs/GLOSSARY.md",
    "docs/THEORY.md",
    "docs/theory/",
    "docs/WRITING.md",
    "README.md",
    "solver/README.md",
    "solver/frame/README.md",
    "solver/frame/testdata/golden/README.md",
    "solver/frame/testdata/fits/README.md",
    "solver/docs/REFERENCE.md",
    "solver/docs/DESIGN.md",
    DESIGN_DIR,
    "solver/docs/AGENDA.md",
    "solver/docs/RUNBOOK.md",
    "solver/docs/runbook/",
    "solver/docs/STATUS.md",
    "solver/tools/certify/README.md",
    "solver/tools/ledger/README.md",
    "solver/noframe/README.md",
    "solver/noframe/docs/",
    "solver/docs/agenda-ideas.md",
    "results/runs.md",
    "solver/notebook/README.md",
    "solver/notebook/index.md",
    "solver/notebook/register/",
    "results/README.md",
    "heuristics/README.md",
];

/// The rewritten docs that speak of the whole game, not of the frame model: the heuristics keep "upper half" and
/// "lower half" for the sets they study, as docs/GLOSSARY.md allows.
const SPEAK_OF_THE_WHOLE_GAME: &[&str] = &["heuristics/README.md"];

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
/// README.md`: the entries are history and keep the words of their time, so the check skips them.  The register's
/// files, the index, the note on the terms changed and the front page are no entries.
fn is_entry(shown: &str) -> bool {
    in_entry_folder(shown).is_some_and(|(_, below)| below == "README.md")
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
        || shown.starts_with(DESIGN_DIR)
        || (is_rewritten(shown) && !SPEAK_OF_THE_WHOLE_GAME.contains(&shown))
}

/// Whether the doc at `shown` (a path from the repository) is in `REWRITTEN`, by name or by its folder (an entry
/// ending in `/`).
fn is_rewritten(shown: &str) -> bool {
    REWRITTEN.iter().any(|&listed| shown == listed || (listed.ends_with('/') && shown.starts_with(listed)))
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

/// The words that docs/GLOSSARY.md replaced (`RETIRED_IN_THE_REWRITTEN_DOCS`, `RETIRED_CASED`) in `lines` (prose: code
/// blanked): `(line number, word, instead)`.
fn glossary_words_in(lines: &[String]) -> Vec<(usize, &'static str, &'static str)> {
    let mut found = retired_in(lines, &[RETIRED_IN_THE_REWRITTEN_DOCS]);
    found.extend(retired_cased_in(lines, RETIRED_CASED));
    found
}

/// The index of the lab notebook, `solver/notebook/index.md`, describes the present: all of it, but for the titles of
/// the entries in its first column, which keep the words of their time.  `text` without those titles, which are cut
/// line by line so that line numbers stay.  The entries themselves (`solver/notebook/NAME/README.md`) are history and
/// are skipped (see `is_entry`), and so is the note that maps the old words (`TERMS_CHANGED`).
fn without_entry_titles(text: &str) -> String {
    let titles_out = |line: &str| match line.starts_with("| [") {
        true => line.splitn(3, '|').nth(2).unwrap_or("").to_owned(),
        false => line.to_owned(),
    };
    text.lines().map(titles_out).collect::<Vec<_>>().join("\n")
}

/// The comments of the Rust source `text`, line by line: the text after `//`, `///` or `//!` outside string and
/// character literals (a `//` in a string, a raw string or a string over several lines is text), and the text of
/// `/* */` comments.  Lines are kept, so that line numbers still mean something; a line with no comment is empty.
fn rust_comments(text: &str) -> Vec<String> {
    #[derive(Clone, Copy)]
    enum Lex {
        Code,
        LineComment,
        BlockComment(usize),
        Str,
        RawStr(usize),
    }
    let chars: Vec<char> = text.chars().collect();
    let at = |i: usize, expected: &str| expected.chars().enumerate().all(|(k, c)| chars.get(i + k) == Some(&c));
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut lines = vec![String::new()];
    let (mut lex, mut i) = (Lex::Code, 0);
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            lines.push(String::new());
            if matches!(lex, Lex::LineComment) {
                lex = Lex::Code;
            }
            i += 1;
            continue;
        }
        match lex {
            Lex::LineComment => {
                lines.last_mut().expect("a line").push(c);
                i += 1;
            }
            Lex::BlockComment(depth) if at(i, "*/") => {
                lex = if depth == 1 { Lex::Code } else { Lex::BlockComment(depth - 1) };
                i += 2;
            }
            Lex::BlockComment(depth) if at(i, "/*") => {
                lex = Lex::BlockComment(depth + 1);
                i += 2;
            }
            Lex::BlockComment(_) => {
                lines.last_mut().expect("a line").push(c);
                i += 1;
            }
            Lex::Str => {
                match c {
                    '"' => lex = Lex::Code,
                    // An escape takes the next character with it, but not the newline of a line continuation.
                    '\\' if chars.get(i + 1) != Some(&'\n') => i += 1,
                    _ => {}
                }
                i += 1;
            }
            Lex::RawStr(hashes) => {
                if c == '"' && (1..=hashes).all(|k| chars.get(i + k) == Some(&'#')) {
                    lex = Lex::Code;
                    i += hashes;
                }
                i += 1;
            }
            Lex::Code if at(i, "//") => {
                lex = Lex::LineComment;
                i += 2;
                while chars.get(i) == Some(&'/') {
                    i += 1;
                }
                if chars.get(i) == Some(&'!') {
                    i += 1;
                }
            }
            Lex::Code if at(i, "/*") => {
                lex = Lex::BlockComment(1);
                i += 2;
            }
            Lex::Code if c == '"' => {
                lex = Lex::Str;
                i += 1;
            }
            Lex::Code if (c == 'r' || at(i, "br")) && !(i > 0 && is_word(chars[i - 1])) => {
                // A raw string, r"..." or r#"..."# (with a b before it for bytes), or a word that starts with r or br.
                let r = if c == 'b' { i + 1 } else { i };
                let hashes = chars[r + 1..].iter().take_while(|&&h| h == '#').count();
                if chars.get(r + 1 + hashes) == Some(&'"') {
                    lex = Lex::RawStr(hashes);
                    i = r + 2 + hashes;
                } else {
                    i += 1;
                }
            }
            Lex::Code if c == '\'' => {
                // A character literal ('a', '"', '\n', '\'') or the start of a lifetime ('a).
                i += match (chars.get(i + 1), chars.get(i + 2)) {
                    (Some('\\'), _) => {
                        4 + chars[(i + 3).min(chars.len())..].iter().position(|&q| q == '\'').unwrap_or(0)
                    }
                    (_, Some('\'')) => 3,
                    _ => 1,
                };
            }
            Lex::Code => i += 1,
        }
    }
    lines
}

/// The comments and docstrings of the Python source `text`, line by line: the lines whose first non-blank character is
/// `#` (but for a shebang), and the text of the docstrings.  A docstring is a string in `"""` that opens a line and is
/// the first statement of the file, or comes after a line that ends in `:` and does not open an `if`, `for`, `else` ...
/// block (that is, after a `def` or a `class`, whose signature may run over several lines).  The other strings in `"""`
/// (a script or a fixture in a test) are code, and so are their lines.  Lines are kept; a line with no comment is
/// empty.
fn python_comments(text: &str) -> Vec<String> {
    const BLOCKS: &[&str] =
        &["if ", "elif ", "else", "for ", "while ", "try", "except", "finally", "with ", "match ", "case "];
    const QUOTES: &str = "\"\"\"";
    let mut lines = Vec::new();
    // While in a string in `"""`: whether it is a docstring.  And the last line of code before this line, if any.
    let (mut inside, mut before): (Option<bool>, Option<&str>) = (None, None);
    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(docstring) = inside {
            let (body, closed) = line.split_once(QUOTES).map_or((line, false), |(body, _)| (body, true));
            lines.push(if docstring { body.to_owned() } else { String::new() });
            if closed {
                inside = None;
            }
        } else if let Some(comment) = trimmed.strip_prefix('#') {
            lines.push(if trimmed.starts_with("#!") { String::new() } else { comment.to_owned() });
        } else if trimmed.is_empty() {
            lines.push(String::new());
        } else {
            let after_quotes = trimmed.trim_start_matches(|c| "rRbBuU".contains(c)).strip_prefix(QUOTES);
            let docstring = after_quotes.filter(|_| {
                before.is_none_or(|code| code.ends_with(':') && !BLOCKS.iter().any(|b| code.starts_with(b)))
            });
            if let Some(body) = docstring {
                // A docstring that closes on its first line has the closing quotes after its text.
                lines.push(body.split(QUOTES).next().unwrap_or("").to_owned());
                inside = (!body.contains(QUOTES)).then_some(true);
                before = Some(QUOTES);
            } else {
                // A string in `"""` that is not a docstring runs on to the next `"""` if this line has an odd number.
                inside = (trimmed.matches(QUOTES).count() % 2 == 1).then_some(false);
                lines.push(String::new());
                before = Some(trimmed);
            }
        }
    }
    lines
}

/// The comments of a source file, line by line: a Rust file's ([`rust_comments`]), a Python script's
/// ([`python_comments`]), or the lines of a shell script whose first non-blank character is `#`.
fn comment_lines(extension: Option<&str>, text: &str) -> Vec<String> {
    match extension {
        Some("rs") => rust_comments(text),
        Some("py") => python_comments(text),
        _ => text
            .lines()
            .map(|line| {
                let trimmed = line.trim_start();
                trimmed.strip_prefix('#').filter(|_| !trimmed.starts_with("#!")).unwrap_or("").to_owned()
            })
            .collect(),
    }
}

/// The prose of a file as lines: a markdown file's text, a source file's comments ([`comment_lines`]), with code spans
/// and fenced blocks blanked.  Lines are kept, so that line numbers still mean something.
fn prose_lines(path: &Path, text: &str) -> Vec<String> {
    let extension = path.extension().and_then(|ext| ext.to_str());
    if extension == Some("md") {
        return without_code(text);
    }
    without_code(&comment_lines(extension, text).join("\n"))
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
/// repository's README and `docs/`, and the READMEs of `heuristics/` and `moniot/`; but for the files that list the old
/// words (`SKIPPED`, `TERMS_CHANGED`) and the notebook's entries.
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
    // The walk skips the test data, but for the READMEs among it that are rewritten.
    let test_data = REWRITTEN.iter().filter(|path| path.contains("/testdata/")).map(|path| root.join(path));
    found.extend(test_data.filter(|path| path.exists()));
    if root.join("README.md").exists() {
        walk(&root.join("docs"), &mut found);
        found.extend(["README.md", "heuristics/README.md", "moniot/README.md"].iter().map(|name| root.join(name)));
    }
    let is_checked = |shown: &str| shown != SKIPPED && shown != TERMS_CHANGED && !is_entry(shown);
    found.into_iter().filter(|path| is_checked(&from_repo(path))).collect()
}

#[test]
fn the_retired_words_stay_out_of_the_guides_and_comments() {
    let mut found = Vec::new();
    let files = vocabulary_files();
    assert!(files.len() > 40, "the check found only {} files: has the layout changed?", files.len());
    for path in files {
        let shown = from_repo(&path);
        let mut text = read(&path);
        if shown == NOTEBOOK_INDEX {
            text = without_entry_titles(&text);
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
        if is_rewritten(&shown) {
            retired.extend(glossary_words_in(&prose));
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
    assert!(found.is_empty(), "words and names the project has retired ({TERMS_CHANGED}):\n{}", found.join("\n"));
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
    let index = "| entry | question |\n| [A hive](a.md) | the hive? | yes |\n| 2026-10 | a hive |";
    assert_eq!(without_entry_titles(index), "| entry | question |\n the hive? | yes |\n| 2026-10 | a hive |");

    let text = "every Board Size, the optimum's score, the `board size` and the \"strong bound\" (quoted)\n\
                the experiments of EXPERIMENTS and [the notebook](EXPERIMENTS.md), and a type checker";
    let found = glossary_words_in(&without_code(text));
    let found: Vec<(usize, &str)> = found.iter().map(|&(at, word, _)| (at, word)).collect();
    assert_eq!(found, [(1, "board size"), (1, "the optimum"), (2, "checker"), (2, "EXPERIMENTS")]);
}

/// The directories, from the repository, whose Rust files' comments use the words of docs/GLOSSARY.md: the source, the
/// examples and the tests of the frame solver and of the prover.
const CODE_DIRS: &[&str] = &[
    "solver/frame/src",
    "solver/frame/examples",
    "solver/frame/tests",
    "solver/noframe/src",
    "solver/noframe/examples",
    "solver/noframe/tests",
];

/// The directories whose scripts' comments (`*.sh`, `*.py`, and the docstrings of the Python) use them too: the tools of
/// the repository and the prover's.
const SCRIPT_DIRS: &[&str] = &["solver/tools", "solver/noframe/tools"];

/// The files whose comments use the glossary's words: those of `CODE_DIRS` and `SCRIPT_DIRS`, but for this one.
fn code_files() -> Vec<PathBuf> {
    let root = repo_dir();
    let below = |dirs: &[&str], extensions: &[&str]| -> Vec<PathBuf> {
        dirs.iter().flat_map(|dir| files_with_extensions(&root.join(dir), extensions)).collect()
    };
    let mut files = below(CODE_DIRS, &["rs"]);
    files.extend(below(SCRIPT_DIRS, &["sh", "py"]));
    files.into_iter().filter(|path| from_repo(path) != SKIPPED).collect()
}

#[test]
fn the_glossarys_words_stay_out_of_the_comments_of_the_code() {
    let files = code_files();
    assert!(files.len() > 150, "the check found only {} files: has the layout changed?", files.len());
    let mut found = Vec::new();
    for path in files {
        for (at, word, instead) in glossary_words_in(&prose_lines(&path, &read(&path))) {
            found.push(format!("{}:{at}: {word:?}, say {instead}", from_repo(&path)));
        }
    }
    assert!(found.is_empty(), "words docs/GLOSSARY.md replaced ({TERMS_CHANGED}), in comments:\n{}", found.join("\n"));
}

#[test]
fn comments_are_found_outside_strings_and_docstrings_are_told_from_other_strings() {
    let rust = [
        r#"let a = "http://x"; // a comment"#,
        r##"let b = "// in a string"; let c = '"'; let d = r#"raw // "quoted""#; // after raw"##,
        "/// doc",
        "//! inner",
        r#"let e = "over"#,
        r#"// two lines"; // after both"#,
        "fn f<'a>(x: &'a str) {} /* block",
        "over lines */ let g = '\\''; // end",
    ]
    .join("\n");
    let expected = [" a comment", " after raw", " doc", " inner", "", " after both", " block", "over lines  end"];
    assert_eq!(rust_comments(&rust)[..8], expected);

    let python = [
        "#!/usr/bin/env python3",
        r#""""Module docstring"#,
        r#"over two lines.""""#,
        "# a comment",
        "def f(",
        "    a,",
        "):",
        r#"    """Doc of f.""""#,
        r#"    text = """not a docstring"#,
        "# not a comment",
        r#"""""#,
        "    if a:",
        r#"        """not a docstring either""""#,
        "    return text  # not taken",
    ]
    .join("\n");
    let expected =
        ["", "Module docstring", "over two lines.", " a comment", "", "", "", "Doc of f.", "", "", "", "", "", ""];
    assert_eq!(python_comments(&python), expected);

    let script = "#!/bin/sh\n# a comment\necho hi # not taken\n  # indented\n";
    assert_eq!(comment_lines(Some("sh"), script), ["", " a comment", "", " indented"]);

    let text = "// a board size, `the optimum` and \"the optimum\"\nlet n = 1; // the checker";
    let found = glossary_words_in(&prose_lines(Path::new("x.rs"), text));
    let found: Vec<(usize, &str)> = found.iter().map(|&(at, word, _)| (at, word)).collect();
    assert_eq!(found, [(1, "board size"), (2, "checker")]);
}

/// The value of the constant `name` at the top of `solver/tools/quick_check.py`.
fn quick_check_limit(name: &str) -> f64 {
    let script = read(repo_dir().join("solver/tools/quick_check.py"));
    let line = script.lines().find_map(|line| line.strip_prefix(&format!("{name} = "))).expect("a constant");
    line.split_whitespace().next().and_then(|value| value.parse().ok()).expect("a number")
}

/// The quick check (`solver/tools/quick_check.py`) passes its own tests, and the design's `testing.md` states its
/// limits and the thresholds of its wait for a quiet machine as the script has them, so that the number in the one is
/// the number in the other.
#[test]
fn the_quick_check_passes_its_own_tests_and_design_states_its_limits() {
    let tests = repo_dir().join("solver/tools/test_quick_check.py");
    let run = std::process::Command::new("python3").arg("-I").arg(tests).output().expect("python3 runs the tests");
    assert!(run.status.success(), "{}{}", String::from_utf8_lossy(&run.stdout), String::from_utf8_lossy(&run.stderr));

    let testing = design_file("testing.md").replace('\n', " ");
    let (target, limit, cpu) =
        (quick_check_limit("TOTAL_TARGET"), quick_check_limit("TOTAL_LIMIT"), quick_check_limit("TEST_LIMIT"));
    let (busy, wait) = ((quick_check_limit("BUSY_LIMIT") * 100.0).round(), quick_check_limit("BUSY_WAIT") / 60.0);
    for stated in [
        format!("more than {limit} s (the target is {target} s;"),
        format!("more than {cpu} s of CPU"),
        format!("more than {busy}% of the CPUs"),
        format!("After {wait} minutes it fails with \"machine busy\""),
    ] {
        assert!(
            testing.contains(&stated),
            "{DESIGN_DIR}testing.md does not say {stated:?}: the script's limits changed"
        );
    }
}

// ---------------------------------------------------------------------------------------------------------
// Citations of headings

/// The documents that a text may name without a path or `.md`: `RUNBOOK, "Certifying U (stage 4)"` cites RUNBOOK.md.
const BARE_DOCS: &[&str] = &["RUNBOOK", "DESIGN", "THEORY", "REFERENCE", "AGENDA", "GLOSSARY", "STATUS", "WRITING"];

/// What may stand between a document and the quoted heading it cites by name, longest first.
const NAME_SEPARATORS: &[&str] = &[", section ", " section ", ", ", ": ", "'s ", " ("];

/// What may stand between a document and the section numbers it cites, longest first.
const NUMBER_SEPARATORS: &[&str] = &[", sections ", " sections ", ", section ", " section ", " §"];

/// The longest heading, in characters, that a citation by name quotes: a longer quotation is a sentence.
const LONGEST_QUOTED_HEADING: usize = 100;

/// The fewest citations by name that the check must find in the repository: fewer means that a pattern broke.  It found
/// 107 on 2026-10-20, and 79 once THEORY.md was split, since a citation of a heading that became a file's title names
/// the file alone.
const FEWEST_BY_NAME: usize = 40;

/// How a text writes the name of a document.
#[derive(Debug, PartialEq)]
struct DocName {
    /// The path or file name as written, or the target of a link (`RUNBOOK` is written `RUNBOOK.md`).
    written: String,
    /// Whether it is the target of a markdown link, which is relative to the citing file.
    link: bool,
}

/// What a citation takes from the document.
#[derive(Debug, PartialEq)]
enum Cited {
    /// A heading, by its text: the text between the quotes.
    Heading(String),
    /// Sections, by their numbers (`9`, `5.4`).
    Sections(Vec<String>),
}

/// A heading or a section cited after the name of a document, with the line where the document is named.
#[derive(Debug, PartialEq)]
struct Citation {
    line: usize,
    doc: DocName,
    cited: Cited,
}

/// A document named in a text, from byte `start` to byte `end` of it.
struct Mention {
    start: usize,
    end: usize,
    doc: DocName,
}

/// A file's text with its wrapped lines joined into one, so that a citation that a line break splits is found.
struct Joined {
    text: String,
    /// Where each line of the file starts in `text`, with the line's number (from 1).
    starts: Vec<(usize, usize)>,
}

impl Joined {
    /// The number of the line of the file that byte `offset` of `text` comes from.
    fn line_at(&self, offset: usize) -> usize {
        self.starts[self.starts.partition_point(|&(start, _)| start <= offset).saturating_sub(1)].1
    }
}

/// What a line of a file says, without what only makes it a line of a comment: its indentation, its comment marker
/// (`//`, `///`, `//!` in Rust, `#` in the scripts) with the spaces after it, a trailing `\` (a string that goes on),
/// and the backslash of an escaped quote.  A markdown line keeps its words.
fn line_content(extension: Option<&str>, line: &str) -> String {
    let line = line.trim();
    let unmarked = match extension {
        Some("md") => return line.to_owned(),
        Some("rs") => line.strip_prefix("//").map_or(line, |rest| {
            let rest = rest.trim_start_matches('/');
            rest.strip_prefix('!').unwrap_or(rest)
        }),
        _ => line.strip_prefix('#').map_or(line, |rest| rest.trim_start_matches('#')),
    };
    unmarked.trim().trim_end_matches('\\').trim_end().replace("\\\"", "\"")
}

/// The text of a file of the given extension as one line per paragraph: a line break, with the comment marker that
/// follows it and its spaces, is one space; a blank line is a break that nothing crosses.  The fenced code blocks of
/// markdown are left out.
fn join_wrapped_lines(extension: Option<&str>, text: &str) -> Joined {
    let lines = if extension == Some("md") { outside_fences(text) } else { text.lines().collect() };
    let mut joined = Joined { text: String::new(), starts: Vec::new() };
    let mut previous_empty = true;
    for (i, line) in lines.into_iter().enumerate() {
        let content = line_content(extension, line);
        if i > 0 {
            joined.text.push(if content.is_empty() || previous_empty { '\n' } else { ' ' });
        }
        joined.starts.push((joined.text.len(), i + 1));
        previous_empty = content.is_empty();
        joined.text.push_str(&content);
    }
    joined
}

/// Whether `c` can be part of the path of a file in a text.
fn is_path_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-')
}

/// The file named by the path that ends with the `.md` at byte `at` of `text`, if a file is named (`*.md` and `.mdx`
/// are not).
fn file_mention_at(text: &str, at: usize) -> Option<Mention> {
    let end = at + ".md".len();
    let before = &text[..at];
    let named = before.ends_with(|c: char| c.is_ascii_alphanumeric() || c == '_');
    if !named || text[end..].starts_with(|c: char| c.is_alphanumeric() || c == '_') {
        return None;
    }
    let start = before.char_indices().rev().take_while(|&(_, c)| is_path_char(c)).last().map_or(at, |(i, _)| i);
    Some(Mention { start, end, doc: DocName { written: text[start..end].to_owned(), link: false } })
}

/// The paths and file names of `text` that end in `.md`.
fn file_mentions(text: &str) -> Vec<Mention> {
    text.match_indices(".md").filter_map(|(at, _)| file_mention_at(text, at)).collect()
}

/// The markdown links of `text` whose target is a `.md` file with no anchor, whatever their text says; the mention ends
/// after the link.
fn link_mentions(text: &str) -> Vec<Mention> {
    let is_file =
        |target: &str| target.ends_with(".md") && !target.contains("://") && !target.contains(char::is_whitespace);
    link_ranges(text)
        .into_iter()
        .filter(|range| is_file(&text[range.clone()]))
        .map(|range| Mention {
            start: range.start,
            end: range.end + 1,
            doc: DocName { written: text[range].to_owned(), link: true },
        })
        .collect()
}

/// The bare names of `BARE_DOCS` in `text`: whole words, not part of a path (`RUNBOOK.md` is a file's name).
fn bare_mentions(text: &str) -> Vec<Mention> {
    let mut found = Vec::new();
    for name in BARE_DOCS {
        for (start, _) in text.match_indices(name) {
            let end = start + name.len();
            let in_a_word_or_path =
                text[..start].ends_with(|c: char| c.is_alphanumeric() || matches!(c, '_' | '/' | '.'));
            let more =
                text[end..].starts_with(|c: char| c.is_alphanumeric() || c == '_') || text[end..].starts_with(".md");
            if !in_a_word_or_path && !more {
                found.push(Mention { start, end, doc: DocName { written: format!("{name}.md"), link: false } });
            }
        }
    }
    found
}

/// Every document named in `text`, in order: by a path or file name, by a link, or by a bare name.
fn mentions(text: &str) -> Vec<Mention> {
    let mut found = file_mentions(text);
    found.extend(link_mentions(text));
    found.extend(bare_mentions(text));
    found.sort_by_key(|mention| mention.start);
    found
}

/// The heading quoted at the start of `text`, in straight or curly double quotes (a heading that has straight quotes in
/// it is quoted in curly ones): at most `LONGEST_QUOTED_HEADING` characters, starting with a capital letter, a digit or
/// a backtick (a quotation that starts with a lower-case word is a key or a phrase).
fn quoted_heading(text: &str) -> Option<String> {
    let (close, inside) =
        text.strip_prefix('"').map(|rest| ('"', rest)).or_else(|| text.strip_prefix('“').map(|rest| ('”', rest)))?;
    let (heading, _) = inside.split_once(close)?;
    let starts_like_a_heading =
        heading.starts_with(|c: char| c.is_uppercase() || c.is_ascii_digit() || c == '`') || is_file_name(heading);
    let short = heading.chars().count() <= LONGEST_QUOTED_HEADING && !heading.contains('\n');
    (starts_like_a_heading && short).then(|| heading.to_owned())
}

/// Whether `text` is one word with a dot in it, a file's name (`bounds.jsonl`), which a heading may be.
fn is_file_name(text: &str) -> bool {
    !text.is_empty() && text.contains('.') && !text.contains(char::is_whitespace)
}

/// The heading cited by name at the start of `rest`, the text that follows a document's name.
fn heading_cited(rest: &str) -> Option<Cited> {
    NAME_SEPARATORS.iter().find_map(|separator| quoted_heading(rest.strip_prefix(separator)?)).map(Cited::Heading)
}

/// The number at the start of `text` (`9`, `5.4`) and what follows it.
fn number_at(text: &str) -> Option<(&str, &str)> {
    let digits = |text: &str| text.find(|c: char| !c.is_ascii_digit()).unwrap_or(text.len());
    let mut end = digits(text);
    if end == 0 {
        return None;
    }
    while text[end..].starts_with('.') && text[end + 1..].starts_with(|c: char| c.is_ascii_digit()) {
        end += 1 + digits(&text[end + 1..]);
    }
    Some(text.split_at(end))
}

/// The section numbers at the start of `text`: `9`, `5.4`, and those that follow `and` (`3 and 4`, `§2 and §5`).
fn section_numbers(text: &str) -> Vec<String> {
    let mut numbers = Vec::new();
    let mut rest = text;
    while let Some((number, after)) = number_at(rest) {
        numbers.push(number.to_owned());
        rest = after.strip_prefix(" and ").map_or("", |next| next.trim_start_matches('§'));
    }
    numbers
}

/// The sections cited by number at the start of `rest`, the text that follows a document's name.
fn sections_cited(rest: &str) -> Option<Cited> {
    let numbers = section_numbers(NUMBER_SEPARATORS.iter().find_map(|separator| rest.strip_prefix(separator))?);
    (!numbers.is_empty()).then_some(Cited::Sections(numbers))
}

/// The headings and sections that the text of a file (of the given extension) cites, after the names of documents
/// (written in code font or not), in order.
fn citations(extension: Option<&str>, text: &str) -> Vec<Citation> {
    let joined = join_wrapped_lines(extension, text);
    mentions(&joined.text)
        .into_iter()
        .filter_map(|mention| {
            let rest = &joined.text[mention.end..];
            let rest = rest.strip_prefix('`').unwrap_or(rest);
            let cited = heading_cited(rest).or_else(|| sections_cited(rest))?;
            Some(Citation { line: joined.line_at(mention.start), doc: mention.doc, cited })
        })
        .collect()
}

/// The `PATH.md#anchor` mentions of the text of a source file: the line, the document and the anchor.
fn code_anchors(extension: Option<&str>, text: &str) -> Vec<(usize, DocName, String)> {
    let joined = join_wrapped_lines(extension, text);
    let is_anchor_char = |c: &char| c.is_alphanumeric() || matches!(c, '-' | '_');
    let mut found = Vec::new();
    for (at, _) in joined.text.match_indices(".md#") {
        let Some(mention) = file_mention_at(&joined.text, at) else { continue };
        let anchor: String = joined.text[at + ".md#".len()..].chars().take_while(is_anchor_char).collect();
        if !anchor.is_empty() {
            found.push((joined.line_at(mention.start), mention.doc, anchor));
        }
    }
    found
}

/// `heading` without its number: "8. Give each task one file" and "5.4 Foo" lose "8. " and "5.4 ".
fn without_number(heading: &str) -> &str {
    let number_end = heading.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(heading.len());
    if heading.starts_with(|c: char| c.is_ascii_digit()) && heading[number_end..].starts_with(' ') {
        heading[number_end..].trim_start()
    } else {
        heading
    }
}

/// `text` as headings are compared with their quotations: without backticks and `*`, and without a final `.` or `,`.
fn plain(text: &str) -> String {
    let text: String = text.chars().filter(|&c| c != '`' && c != '*').collect();
    text.trim().trim_end_matches(['.', ',']).to_owned()
}

/// Whether the quotation `quoted` names `heading`: it is the heading's text, with its number or without.
fn names_heading(quoted: &str, heading: &str) -> bool {
    let quoted = plain(quoted);
    quoted == plain(heading) || quoted == plain(without_number(heading))
}

/// Whether `heading` is numbered `number`: its text starts with it, then `.` or a space.
fn has_number(heading: &str, number: &str) -> bool {
    heading.strip_prefix(number).is_some_and(|rest| rest.starts_with(['.', ' ']))
}

/// What the citations are checked against: the repository's markdown files, and the headings and anchors of the
/// documents cited so far.
struct DocSet {
    markdown: Vec<PathBuf>,
    headings: BTreeMap<PathBuf, Vec<String>>,
    anchors: BTreeMap<PathBuf, BTreeSet<String>>,
}

impl DocSet {
    fn new() -> DocSet {
        DocSet { markdown: markdown_files(), headings: BTreeMap::new(), anchors: BTreeMap::new() }
    }

    /// The markdown file called `name` that is the only one of that name, or why there is not one.
    fn only_file_named(&self, name: &str) -> Result<PathBuf, String> {
        let same: Vec<&PathBuf> =
            self.markdown.iter().filter(|path| path.file_name().is_some_and(|n| n == name)).collect();
        match same.as_slice() {
            [only] => Ok((*only).clone()),
            [] => Err(format!("there is no file {name}: write its path from the repository's top")),
            several => Err(format!("{name} names {} files: write its path from the repository's top", several.len())),
        }
    }

    /// The file that `doc`, written in the file `citing`, names, or why it names none.  A link is relative to the
    /// citing file; a path with a `/` is from the repository's top, else relative to the citing file; a bare name is
    /// the file of that name beside the citing file, else the only one of the markdown files.
    fn resolve(&self, citing: &Path, doc: &DocName) -> Result<PathBuf, String> {
        let written = doc.written.as_str();
        let beside = citing.parent().expect("a file has a directory").join(written);
        let from_top = repo_dir().join(written);
        let found = if doc.link || written.starts_with('.') {
            beside
        } else if written.contains('/') {
            if from_top.exists() {
                from_top
            } else {
                beside
            }
        } else if beside.exists() {
            beside
        } else {
            return self.only_file_named(written);
        };
        found
            .canonicalize()
            .map_err(|_| format!("there is no file {written}: write its path from the repository's top"))
    }

    /// The headings of the markdown file `doc`.
    fn headings_of(&mut self, doc: &Path) -> &[String] {
        self.headings
            .entry(doc.to_path_buf())
            .or_insert_with(|| headings(&read(doc)).into_iter().map(str::to_owned).collect())
    }

    /// What is wrong with `citation`, written in the file `citing`, if anything.
    fn citation_problem(&mut self, citing: &Path, citation: &Citation) -> Option<String> {
        let doc = match self.resolve(citing, &citation.doc) {
            Ok(doc) => doc,
            Err(problem) => return Some(problem),
        };
        let shown = from_repo(&doc);
        let headings = self.headings_of(&doc);
        let is_named = |quoted: &str| headings.iter().any(|heading| names_heading(quoted, heading));
        let is_numbered = |number: &str| headings.iter().any(|heading| has_number(heading, number));
        match &citation.cited {
            Cited::Heading(quoted) if !is_named(quoted) => Some(format!(
                "{shown} has no heading \"{quoted}\": cite a heading by its text, with its number or without"
            )),
            Cited::Sections(numbers) => numbers.iter().find(|number| !is_numbered(number)).map(|number| {
                format!(
                    "{shown} has no heading numbered {number}: cite the heading by its name, as docs/WRITING.md, \
                     rule 8, says"
                )
            }),
            Cited::Heading(_) => None,
        }
    }

    /// What is wrong with the `anchor` of `doc`, written in the file `citing`, if anything.
    fn anchor_problem(&mut self, citing: &Path, doc: &DocName, anchor: &str) -> Option<String> {
        let path = match self.resolve(citing, doc) {
            Ok(path) => path,
            Err(problem) => return Some(problem),
        };
        let known = self.anchors.entry(path.clone()).or_insert_with(|| anchors(&read(&path)));
        (!known.contains(anchor)).then(|| format!("no heading of {} has the anchor #{anchor}", from_repo(&path)))
    }
}

/// The markdown files that cite docs and keep rule 8's limits: those of `markdown_files` but the notebook's entries
/// (which are records), and the top-level CLAUDE.md.
fn guide_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = markdown_files().into_iter().filter(|path| !is_entry(&from_repo(path))).collect();
    files.push(repo_dir().join("CLAUDE.md"));
    files
}

/// The files that cite docs: the guides, the code and the scripts (`code_files`, and the build script, the Dockerfiles
/// of the tools and the heuristics' Python).
fn citing_files() -> Vec<PathBuf> {
    let root = repo_dir();
    let mut files = guide_files();
    files.extend(code_files());
    files.push(root.join("solver/frame/build.rs"));
    files.extend(files_where(&root.join("solver/tools"), &|path| {
        path.file_name().is_some_and(|name| name.to_string_lossy().starts_with("Dockerfile"))
    }));
    files.extend(files_with_extensions(&root.join("heuristics"), &["py"]));
    files
}

/// What the check found in the files: how many citations by name, and what is wrong.  Citations by number have no
/// floor: rule 8's splits remove them on purpose, and the unit tests below check that the pattern still finds them.
#[derive(Default)]
struct Found {
    by_name: usize,
    problems: Vec<String>,
}

/// Checks the citations of headings in the file `file`, and its anchors if it is a source file (the link test checks
/// those of the markdown).
fn check_citations(docs: &mut DocSet, file: &Path, found: &mut Found) {
    let text = read(file);
    let extension = file.extension().and_then(|ext| ext.to_str());
    let at = |line: usize, problem: String| format!("{}:{line}: {problem}", from_repo(file));
    for citation in citations(extension, &text) {
        if matches!(citation.cited, Cited::Heading(_)) {
            found.by_name += 1;
        }
        found.problems.extend(docs.citation_problem(file, &citation).map(|problem| at(citation.line, problem)));
    }
    if extension != Some("md") {
        for (line, doc, anchor) in code_anchors(extension, &text) {
            found.problems.extend(docs.anchor_problem(file, &doc, &anchor).map(|problem| at(line, problem)));
        }
    }
}

/// Every heading that a guide, a comment or a script cites by name (`DESIGN.md, "Testing"`) or by number
/// (`DESIGN.md, section 9`) exists, and so does every `#anchor` that the code cites.
#[test]
fn every_heading_cited_by_name_or_number_exists_and_so_does_every_anchor_cited_in_the_code() {
    if !repo_dir().join("README.md").exists() {
        eprintln!("citations not checked: this is not a full checkout of the repository (no README.md at its top)");
        return;
    }
    let (mut docs, mut found) = (DocSet::new(), Found::default());
    for file in citing_files() {
        check_citations(&mut docs, &file, &mut found);
    }
    assert!(found.problems.is_empty(), "broken citations of headings:\n{}", found.problems.join("\n"));
    let by_name = found.by_name;
    assert!(by_name >= FEWEST_BY_NAME, "the check found only {by_name} citations by name: has their syntax changed?");
}

/// The citations of `text` as short lines: the line, the document, and the heading in quotes or the sections after `§`.
fn citations_shown(extension: Option<&str>, text: &str) -> Vec<String> {
    let shown = |c: &Citation| {
        let cited = match &c.cited {
            Cited::Heading(heading) => format!("\"{heading}\""),
            Cited::Sections(numbers) => format!("§{}", numbers.join("+")),
        };
        format!("{} {}{} {cited}", c.line, c.doc.written, if c.doc.link { " (link)" } else { "" })
    };
    citations(extension, text).iter().map(shown).collect()
}

/// A document is named by a path, a file name, a link or a bare name, and cited by name (after each separator, in
/// either quotes) or by number; what is no citation (a key, a fence, a blank line between) is left out.
#[test]
fn citations_are_found_in_every_form_of_markdown() {
    let text = [
        "See DESIGN.md, \"Testing\" and [the runbook](RUNBOOK.md), section 4.",
        "The docs/THEORY.md's “Longest games”, REFERENCE.md (\"A results line\"), AGENDA.md: \"A5. Decompose\",",
        "STATUS.md section \"Unwrapped\" and RUNBOOK, \"Dry run in Docker\".  DESIGN.md §2 and §5, but",
        "THEORY.md, sections 3 and 4.2; WRITING.md, section 8.  Not [x](DESIGN.md#testing), \"Testing\",",
        "nor README.md, \"a key\", nor *.md, \"Star\", nor `DESIGN.md`, \"Testing\" [the design](DESIGN.md);",
        "results/README.md, \"bounds.jsonl\", but not REFERENCE.md, \"a two-word key\".",
        "```",
        "THEORY.md, \"In a fence\"",
        "```",
        "* A list item that cites DESIGN.md,",
        "  \"Same search\", and RUNBOOK",
        "",
        "\"Not joined\", nor DESIGN.md,",
        "",
        "\"Over a blank line\".",
    ]
    .join("\n");
    assert_eq!(
        citations_shown(Some("md"), &text),
        [
            "1 DESIGN.md \"Testing\"",
            "1 RUNBOOK.md (link) §4",
            "2 docs/THEORY.md \"Longest games\"",
            "2 REFERENCE.md \"A results line\"",
            "2 AGENDA.md \"A5. Decompose\"",
            "3 STATUS.md \"Unwrapped\"",
            "3 RUNBOOK.md \"Dry run in Docker\"",
            "3 DESIGN.md §2+5",
            "4 THEORY.md §3+4.2",
            "4 WRITING.md §8",
            "5 DESIGN.md \"Testing\"",
            "6 results/README.md \"bounds.jsonl\"",
            "10 DESIGN.md \"Same search\"",
        ]
    );
}

/// A citation that a line break splits is found, over `//!`, `///` and `//` lines, over `#` lines and over a string
/// that goes on, at the line where its document is named; a blank line is a break that nothing crosses.
#[test]
fn a_citation_wrapped_over_two_comment_lines_is_found_at_the_line_where_it_starts() {
    let rust = [
        "//! The theory is in docs/THEORY.md, \"The frame",
        "//! model is exact\", and DESIGN.md, section",
        "/// 9 has the limits; see also solver/docs/RUNBOOK.md,",
        "    // \"Certifying U (stage 4)\" (a comment inside a function).",
        "let m = \"see DESIGN.md, \\\"Same search\\\"\";",
        "let n = \"see DESIGN.md, \\",
        "         section 4\";",
    ]
    .join("\n");
    assert_eq!(
        citations_shown(Some("rs"), &rust),
        [
            "1 docs/THEORY.md \"The frame model is exact\"",
            "2 DESIGN.md §9",
            "3 solver/docs/RUNBOOK.md \"Certifying U (stage 4)\"",
            "5 DESIGN.md \"Same search\"",
            "6 DESIGN.md §4",
        ]
    );
    let script = [
        "#!/usr/bin/env python3",
        "# See solver/docs/DESIGN.md, \"Same",
        "# search\" and RUNBOOK.md, sections 2.5",
        "# and 2.7.",
        "",
        "# DESIGN.md,",
        "",
        "# \"Not joined over a blank line\"",
    ]
    .join("\n");
    assert_eq!(
        citations_shown(Some("py"), &script),
        ["2 solver/docs/DESIGN.md \"Same search\"", "3 RUNBOOK.md §2.5+2.7"]
    );
}

/// A `PATH.md#anchor` is found in a comment, wrapped or not, but for an empty anchor and a longer extension.
#[test]
fn anchors_cited_in_the_code_are_found_wrapped_or_not() {
    let text = "/// See [the ledger](../docs/THEORY.md#the-ledger-bounds-between-boards) and\n\
                /// DESIGN.md#testing-1.  Not README.md# or notes.mdx#x.";
    let found: Vec<(usize, String, String)> =
        code_anchors(Some("rs"), text).into_iter().map(|(line, doc, anchor)| (line, doc.written, anchor)).collect();
    let expected = [(1, "../docs/THEORY.md", "the-ledger-bounds-between-boards"), (2, "DESIGN.md", "testing-1")];
    assert_eq!(found, expected.map(|(line, doc, anchor)| (line, doc.to_owned(), anchor.to_owned())));
}

/// A quotation names a heading by its text, with its number or without, ignoring backticks, `*` and a final `.` or `,`
/// (and counting case); a section is named by the number its heading starts with; a quotation is a heading only if it
/// starts like one (or is a file's name) and is not too long; the headings of a file are those outside fenced code.
#[test]
fn headings_are_quoted_with_their_number_or_without_and_sections_are_named_by_number() {
    assert!(names_heading("Tactics", "5. Tactics") && names_heading("5. Tactics", "5. Tactics"));
    assert!(names_heading("Same search", "5.4. Same search") && names_heading("Same search", "5.4 Same search"));
    assert!(names_heading("Per-game pipeline (solver)", "7. Per-game pipeline (`solver`)"));
    assert!(names_heading("`taxman` runs.", "**`taxman`** runs") && names_heading("A, B,", "A, B"));
    assert!(names_heading("The span certificate (♦)", "The span certificate (♦)"));
    assert!(!names_heading("same search", "Same search"), "case counts");
    assert!(!names_heading("Extended mode", "Extended mode: one-sided candidates"), "the whole heading");
    assert!(!names_heading("2. Tactics", "Tactics"), "a number the heading does not have");

    assert!(has_number("9. Testing", "9") && has_number("5.4 Foo", "5.4") && has_number("5.4. Foo", "5.4"));
    assert!(!has_number("10. Foo", "1") && !has_number("Testing", "9") && !has_number("5.4 Foo", "5.5"));

    let curly = "“What \"optimal\" means”, and more";
    assert_eq!(quoted_heading(curly).as_deref(), Some("What \"optimal\" means"));
    assert_eq!(quoted_heading("\"9. Testing\" x").as_deref(), Some("9. Testing"));
    assert_eq!(quoted_heading("\"`bounds.jsonl`\"").as_deref(), Some("`bounds.jsonl`"));
    assert_eq!(quoted_heading("\"bounds.jsonl\"").as_deref(), Some("bounds.jsonl"), "a file's name may be a heading");
    assert_eq!(quoted_heading("\"proof\""), None, "a lower-case word is a key, not a heading");
    assert_eq!(quoted_heading("\"Open"), None);
    let long = |length: usize| format!("\"{}\"", "A".repeat(length));
    assert_eq!(quoted_heading(&long(LONGEST_QUOTED_HEADING)).map(|heading| heading.len()), Some(100));
    assert_eq!(quoted_heading(&long(LONGEST_QUOTED_HEADING + 1)), None, "a quotation of a sentence");

    let text = "# Title\n```\n## Not\n```\n## 2. Yes ##\n#Not\n### Also";
    assert_eq!(headings(text), ["Title", "2. Yes ##", "Also"]);
}

// ---------------------------------------------------------------------------------------------------------
// Sizes

/// The documents that rule 8 of docs/WRITING.md would have split and that are still whole, by their paths from the
/// repository.  A document leaves the list when it is split, and its old file, now an index, joins `INDEXES`.
const STILL_TO_SPLIT: &[&str] = &[];

/// The old files of the documents that were split, by their paths from the repository: each is now an index, a line a
/// file, and stays under the index limit.
const INDEXES: &[&str] = &[
    "docs/THEORY.md",
    "solver/noframe/README.md",
    "solver/docs/DESIGN.md",
    "solver/docs/RUNBOOK.md",
    "solver/notebook/README.md",
];

/// The words of `text`, as `wc -w` counts them: its runs of characters between white space.
fn word_count(text: &str) -> usize {
    text.split_whitespace().count()
}

/// The number that follows `before` in `text` (a sentence on one line), written with or without commas:
/// `limit_after("stays under 6,000 words", "under ")` is 6000.
fn limit_after(text: &str, before: &str) -> Option<usize> {
    let after = text.split_once(before)?.1;
    let number: String = after.chars().take_while(|c| c.is_ascii_digit() || *c == ',').collect();
    number.trim_end_matches(',').replace(',', "").parse().ok()
}

/// The limits of rule 8 of docs/WRITING.md, in words: of a file, and of the index that a split document leaves.
fn word_limits() -> (usize, usize) {
    let writing = read(repo_dir().join("docs/WRITING.md"));
    let heading = "## 8. Give each task one file";
    assert!(writing.contains(&format!("\n{heading}\n")), "docs/WRITING.md has lost its section {heading:?}");
    let rule = section(&writing, heading).replace('\n', " ");
    let limit = |before: &str| {
        let message =
            format!("docs/WRITING.md, rule 8, no longer says {before:?} and a number: the size test reads it");
        limit_after(&rule, before).unwrap_or_else(|| panic!("{message}"))
    };
    (limit("A file stays under "), limit("stays as its index, under "))
}

/// Whether the file at `shown` (a path from the repository) lies in the folder of an entry of the lab notebook: the
/// entry's README.md and the files beside it, which its scripts wrote and which are committed as they were.  The front
/// page, the index, the note on the terms changed and the register's files are guides and keep the limits.
fn is_in_an_entry(shown: &str) -> bool {
    in_entry_folder(shown).is_some()
}

/// The word counts of the guides that rule 8 holds to the file limit, with their paths from the repository: those of
/// `guide_files`, but the files in the folders of the notebook's entries (which are records).
fn guide_sizes() -> Vec<(String, usize)> {
    let shown_files = guide_files().into_iter().map(|path| (from_repo(&path), path));
    shown_files
        .filter(|(shown, _)| !is_in_an_entry(shown))
        .map(|(shown, path)| (shown, word_count(&read(&path))))
        .collect()
}

/// The size of the file `shown` (a path from the repository) in words, if it exists.
fn words_in(shown: &str) -> Option<usize> {
    std::fs::read_to_string(repo_dir().join(shown)).ok().map(|text| word_count(&text))
}

/// What is wrong with the sizes of the documents, as messages, given the limits of a file and of an index.
fn size_problems(file_limit: usize, index_limit: usize) -> Vec<String> {
    let mut problems = Vec::new();
    for (shown, words) in guide_sizes() {
        if words >= file_limit && !STILL_TO_SPLIT.contains(&shown.as_str()) {
            problems.push(format!(
                "{shown} has {words} words, over the limit of {file_limit} of docs/WRITING.md, rule 8: split it by task"
            ));
        }
    }
    for shown in STILL_TO_SPLIT {
        match words_in(shown) {
            None => problems
                .push(format!("{shown} is in STILL_TO_SPLIT but there is no such file: remove it from STILL_TO_SPLIT")),
            Some(words) if words < file_limit => problems.push(format!(
                "{shown} has {words} words, under the limit of {file_limit}: remove it from STILL_TO_SPLIT"
            )),
            Some(_) => {}
        }
    }
    for shown in INDEXES {
        match words_in(shown) {
            None => problems.push(format!("{shown} is in INDEXES but there is no such file: remove it from INDEXES")),
            Some(words) if words >= index_limit => problems.push(format!(
                "{shown} is an index (INDEXES) of {words} words, over the limit of {index_limit} of \
                 docs/WRITING.md, rule 8: an index holds a line a file; move the rest into the files of its folder"
            )),
            Some(_) => {}
        }
    }
    problems
}

/// Every guide but the notebook's entries keeps rule 8's limit of words, but those still to be split, and the indexes
/// of the split documents keep the limit of an index.
#[test]
fn documents_keep_the_size_limits_of_rule_8() {
    if !repo_dir().join("README.md").exists() {
        eprintln!("sizes not checked: this is not a full checkout of the repository (no README.md at its top)");
        return;
    }
    let (file_limit, index_limit) = word_limits();
    assert!(file_limit > index_limit, "rule 8 lets a file be {file_limit} words and an index {index_limit}");
    let problems = size_problems(file_limit, index_limit);
    assert!(problems.is_empty(), "documents that break rule 8 of docs/WRITING.md:\n{}", problems.join("\n"));
}

/// The limits are read from the rule's sentences, and words are counted as `wc -w` does.
#[test]
fn the_limits_are_read_from_the_rule_and_words_are_counted_as_wc_does() {
    let rule = "A file stays under 6,000 words, and should stay under 5,000.  \
                The old file stays as its index, under 1,000 words.";
    assert_eq!(limit_after(rule, "A file stays under "), Some(6000));
    assert_eq!(limit_after(rule, "stays as its index, under "), Some(1000));
    assert_eq!(limit_after(rule, "stays as its table, under "), None, "a sentence that is gone");
    assert_eq!(limit_after("under 750, with a line", "under "), Some(750));
    assert_eq!(word_count("a  b\n\tc—d \n"), 3);
}
