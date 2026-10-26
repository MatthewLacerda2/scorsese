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
