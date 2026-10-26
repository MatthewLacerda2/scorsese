//! Reading a manifest by hand: a name, and every dependency table.

use scorsese_lint::rules::boundary::manifest::parse;

#[test]
fn every_dependency_table_is_read_and_nothing_else() {
    let text = r##"
[package]
name = "scorsese-x"
version = "0.1.0"

[package.metadata.docs]
all-features = true

[dependencies]
scorsese-core = { path = "../core" }
serde = "1"
proptest = { version = "1", features = [
    "std",
    "]", "]",
    nested = "not a key",
] }
# retired = "1"
quoted = { version = "1", features = ["#", "["] }
commented = "1" # an open [ in a comment
after = "1"

[dev-dependencies]
renamed = { package = "scorsese-render", path = "../render" }

[target.'cfg(target_os = "linux")'.dependencies]
libc = "0.2"

[dependencies.ureq]
version = "3"

[features]
test-support = []
"##;
    let manifest = parse(text);
    assert_eq!(manifest.name.as_deref(), Some("scorsese-x"));
    let names: Vec<(&str, usize)> = manifest
        .dependencies
        .iter()
        .map(|dep| (dep.name.as_str(), dep.line))
        .collect();
    assert_eq!(
        names,
        [
            ("scorsese-core", 10),
            ("serde", 11),
            ("proptest", 12),
            ("quoted", 18),
            ("commented", 19),
            ("after", 20),
            ("scorsese-render", 23),
            ("libc", 26),
            ("ureq", 28),
        ]
    );
}

#[test]
fn a_manifest_without_a_package_has_no_name() {
    assert_eq!(parse("[workspace]\nmembers = []\n").name, None);
}

#[test]
fn a_dotted_key_names_the_crate_before_its_first_dot() {
    let text = r#"
[dependencies]
strsim.workspace = true
scorsese-render.path = "../render"
serde . version = "1"
"dotted.name" = "1"
"quoted".workspace = true
alias.package = "scorsese-core"
"#;
    let names: Vec<String> = parse(text)
        .dependencies
        .into_iter()
        .map(|dep| dep.name)
        .collect();
    assert_eq!(
        names,
        [
            "strsim",
            "scorsese-render",
            "serde",
            "dotted.name",
            "quoted",
            "scorsese-core",
        ]
    );
}

fn dependencies(text: &str) -> Vec<(String, usize)> {
    parse(text)
        .dependencies
        .into_iter()
        .map(|dep| (dep.name, dep.line))
        .collect()
}

#[test]
fn a_rename_is_the_string_and_not_its_comment_or_quotes() {
    let text = r#"
[dependencies]
core.package = "scorsese-core" # renamed
render.package = 'scorsese-render'
zimmer."package" = "scorsese-zimmer"
'literal.key'.workspace = true
"#;
    assert_eq!(
        dependencies(text),
        [
            ("scorsese-core".to_string(), 3),
            ("scorsese-render".to_string(), 4),
            ("scorsese-zimmer".to_string(), 5),
            ("literal.key".to_string(), 6),
        ]
    );
}

#[test]
fn the_lines_of_one_key_are_one_dependency_at_its_first_line() {
    let text = r#"
[dependencies]
ureq.version = "2"
ureq.default-features = false
alias.path = "../zimmer"
alias.package = "scorsese-zimmer"

[dev-dependencies]
ureq.version = "2"

[dependencies.named]
package = "scorsese-core"
path = "../core"
"#;
    assert_eq!(
        dependencies(text),
        [
            ("ureq".to_string(), 3),
            ("scorsese-zimmer".to_string(), 5),
            ("ureq".to_string(), 9),
            ("scorsese-core".to_string(), 11),
        ]
    );
}

#[test]
fn a_header_with_a_comment_still_opens_its_table() {
    let text =
        "[dependencies] # runtime\nserde = \"1\"\n[dependencies.ureq] # http\nversion = \"3\"\n";
    assert_eq!(
        dependencies(text),
        [("serde".to_string(), 2), ("ureq".to_string(), 3)]
    );
}

#[test]
fn a_hash_inside_a_quoted_header_is_not_its_comment() {
    let text = "[target.'cfg(feature = \"a#b\")'.dependencies] # note\nlibc = \"0.2\"\n";
    assert_eq!(dependencies(text), [("libc".to_string(), 2)]);
}
