//! The server logs through `tracing`: a print macro under `crates/server/src`
//! is flagged, the same macro anywhere else, in a comment or inside a longer
//! name is not, and an allow with a reason excuses it.

use std::path::Path;

use scorsese_lint::rules::printing::{RULE, check};

const SERVER: &str = "crates/server/src/http/error.rs";

#[test]
fn a_print_in_the_server_is_flagged_on_its_line() {
    for print in [
        "eprintln!(\"x\")",
        "println!(\"{x}\")",
        "eprint!(\"x\")",
        "print!(\"x\")",
        "dbg!(x)",
        "let u = \"http://x\"; eprintln!(\"{u}\")",
        "let q = '\"'; eprintln!(\"{q}\")",
        "let half = a / 2; eprintln!(\"{half}\")",
    ] {
        let text = format!("fn a() {{\n    {print};\n}}\n");
        let [finding] = &check(Path::new(SERVER), &text)[..] else {
            panic!("expected one finding for {print}");
        };
        assert_eq!(finding.line, Some(2));
        assert_eq!(finding.rule, RULE);
        assert!(finding.message.contains("tracing"), "{}", finding.message);
    }
}

#[test]
fn a_print_elsewhere_or_in_a_comment_or_a_longer_name_passes() {
    let print = "fn a() { eprintln!(\"x\"); }\n";
    assert!(check(Path::new("crates/cli/src/main.rs"), print).is_empty());
    assert!(check(Path::new("crates/server/tests/serve.rs"), print).is_empty());
    let quiet =
        "//! Once `eprintln!` logged here.\nfn a() { my_println!(x); tracing::error!(\"x\"); }\n";
    assert!(check(Path::new(SERVER), quiet).is_empty());
    let said = "fn a() { tracing::warn!(\"never eprintln!(x) here, \\\" nor println!(y)\"); }\n";
    assert!(check(Path::new(SERVER), said).is_empty());
    let after = "fn a() { tracing::warn!(\"x\"); // once eprintln!(x)\n}\n";
    assert!(check(Path::new(SERVER), after).is_empty());
}

#[test]
fn an_allow_with_a_reason_excuses_the_line_below_it() {
    let text = format!(
        "// lint: allow({RULE}) the operator's answer, on stdout\nprintln!(\"{{output}}\");\n"
    );
    assert!(check(Path::new(SERVER), &text).is_empty());
    let text = format!("// lint: allow({RULE})\nprintln!(\"{{output}}\");\n");
    assert_eq!(check(Path::new(SERVER), &text).len(), 1);
}
