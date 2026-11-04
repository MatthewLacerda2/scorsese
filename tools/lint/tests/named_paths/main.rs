//! A repository path a comment or a doc names must exist: a missing one is
//! flagged with its line, an existing one and an allowed one pass, and string
//! literals, templates and URLs are never read as paths.

mod tree;

use std::path::Path;

use scorsese_lint::rules::named_paths::{Known, RULE, check, named_in};

/// The tree the cases below name paths in.
fn known() -> Known {
    Known::new(
        [
            "crates/cli/tests/suite/main.rs",
            "crates/render/src/run/mod.rs",
            "docs/mcp.md",
            ".github/scripts/ci-ok.py",
            "deploy/compose.yaml",
            "app/Cargo.toml",
        ]
        .map(Path::new),
    )
}

/// The missing paths `check` flags in `text`, read as the file at `path`.
fn flagged(path: &str, text: &str) -> Vec<(usize, String)> {
    check(Path::new(path), text, &known())
        .into_iter()
        .map(|finding| {
            assert_eq!(finding.rule, RULE);
            let line = finding.line.unwrap_or_default();
            let named = finding.message.split('`').nth(1).unwrap_or_default();
            (line, named.to_owned())
        })
        .collect()
}

#[test]
fn a_comment_naming_a_missing_file_is_flagged_with_its_line() {
    let text = "fn a() {}\n/// Tested in `crates/cli/tests/synth/metadata.rs`.\n";
    let found = flagged("crates/cli/src/a.rs", text);
    assert_eq!(found, [(2, "crates/cli/tests/synth/metadata.rs".into())]);
    let [finding] = &check(Path::new("a.rs"), text, &known())[..] else {
        panic!("expected one finding");
    };
    assert!(finding.message.contains("repoint"), "{}", finding.message);
}

#[test]
fn a_crate_relative_path_resolves_under_crates() {
    assert_eq!(
        flagged(
            "a.rs",
            "// see render/src/run.rs and cli/tests/suite/main.rs\n"
        ),
        [(1, "crates/render/src/run.rs".into())]
    );
}

#[test]
fn existing_files_and_directories_pass() {
    let text = "// crates/render/src/run/ and docs/mcp.md, .github/scripts/ci-ok.py:12.\n";
    assert_eq!(flagged("a.rs", text), []);
    assert_eq!(flagged("docs/a.md", "See [mcp](docs/mcp.md#tools).\n"), []);
}

#[test]
fn an_allow_with_a_reason_excuses_the_line() {
    let rust = format!("// lint: allow({RULE}) an example\n// docs/invented.md\n");
    assert_eq!(flagged("a.rs", &rust), []);
    let markdown = format!("<!-- lint: allow({RULE}) created by setup -->\ndeploy/.env\n");
    assert_eq!(flagged("docs/a.md", &markdown), []);
    assert_eq!(
        flagged("docs/a.md", "deploy/.env\n"),
        [(1, "deploy/.env".into())]
    );
}

#[test]
fn an_allow_reaches_only_the_line_below_it() {
    let rust = format!("// lint: allow({RULE}) an example\n// prose\n// docs/invented.md\n");
    assert_eq!(flagged("a.rs", &rust), [(3, "docs/invented.md".into())]);
}

#[test]
fn a_path_git_ignores_counts_as_existing() {
    let mut known = known();
    known.ignore(["deploy/.env", "app/target/"]);
    known.ignore([".claude/skill/x.md"]);
    let text = "deploy/.env, app/target/, deploy/.envrc and .claude/skill/x.md\n";
    let found = check(Path::new("docs/a.md"), text, &known);
    let named: Vec<_> = found.iter().map(|f| f.message.split('`').nth(1)).collect();
    assert_eq!(named, [Some("deploy/.envrc"), Some(".claude/skill/x.md")]);
}

#[test]
fn only_comments_are_read_in_code() {
    let rust = "let path = \"docs/invented.md\";\n";
    assert_eq!(flagged("a.rs", rust), []);
    let python = "PATH = 'docs/invented.md'\n# docs/gone.md\n";
    assert_eq!(
        flagged(".github/scripts/a.py", python),
        [(2, "docs/gone.md".into())]
    );
    let yaml = "run: cat docs/invented.md\n# docs/gone.md\n";
    assert_eq!(
        flagged(".github/workflows/ci.yml", yaml),
        [(2, "docs/gone.md".into())]
    );
    let make = "size: ## the docs/gone.md gate\n\tcat docs/invented.md\n";
    assert_eq!(flagged("Makefile", make), [(1, "docs/gone.md".into())]);
}

#[test]
fn files_outside_the_scope_are_not_read() {
    assert_eq!(flagged("web/src/a.ts", "// docs/gone.md\n"), []);
    assert_eq!(flagged("crates/core/notes.md", "docs/gone.md\n"), []);
    assert_eq!(flagged("crates/core/CLAUDE.md", "docs/gone.md\n").len(), 1);
}

#[test]
fn templates_globs_urls_and_bare_words_are_not_paths() {
    for line in [
        "crates/<crate>/src/lib.rs",
        "crates/render/src/{audio,say}/",
        "crates/*/tests/**",
        "web/src/…/x.ts",
        "https://github.com/o/r/blob/main/docs/gone.md",
        "/home/user/docs/gone.md",
        "../docs/gone.md",
        "scorsese/docs/gone.md",
        "the app/web split, tools/lint",
        "$ROOT/docs/gone.md",
    ] {
        assert_eq!(named_in(line, &known()), Vec::<String>::new(), "{line}");
    }
    for line in [
        "(docs/gone.md).",
        "**docs/gone.md**:",
        "<docs/gone.md>",
        "docs/gone.md?",
        "_docs/gone.md_",
    ] {
        assert_eq!(named_in(line, &known()), ["docs/gone.md"], "{line}");
    }
}
