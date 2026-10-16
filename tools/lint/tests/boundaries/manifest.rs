//! Reading a manifest by hand: a name, and every dependency table.

use scorsese_lint::rules::boundary::manifest::parse;

#[test]
fn every_dependency_table_is_read_and_nothing_else() {
    let text = r#"
[package]
name = "scorsese-x"
version = "0.1.0"

[dependencies]
scorsese-core = { path = "../core" }
serde = "1"
proptest = { version = "1", features = [
    "std",
    nested = "not a key",
] }

[dev-dependencies]
renamed = { package = "scorsese-render", path = "../render" }

[target.'cfg(unix)'.dependencies]
libc = "0.2"

[dependencies.ureq]
version = "3"

[features]
test-support = []
"#;
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
            ("scorsese-core", 7),
            ("serde", 8),
            ("proptest", 9),
            ("scorsese-render", 15),
            ("libc", 18),
            ("ureq", 20),
        ]
    );
}

#[test]
fn a_manifest_without_a_package_has_no_name() {
    assert_eq!(parse("[workspace]\nmembers = []\n").name, None);
}
