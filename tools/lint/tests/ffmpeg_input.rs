//! A bare `"-i"` in a crate that runs ffmpeg is flagged; one elsewhere, in
//! a comment or inside a longer literal is not, and an allow with a reason
//! excuses it.

use std::path::Path;

use scorsese_lint::rules::ffmpeg_input::{RULE, check};

const RENDER: &str = "crates/render/src/frames/mod.rs";

#[test]
fn a_bare_input_in_a_crate_that_runs_ffmpeg_is_flagged_on_its_line() {
    for input in [
        "command.arg(\"-i\").arg(file)",
        "command.args([\"-nostdin\", \"-v\", \"error\", \"-i\"]).arg(file)",
        "let u = \"http://x\"; command.args([\"-i\", u])",
    ] {
        let text = format!("fn a() {{\n    {input};\n}}\n");
        for path in [
            RENDER,
            "crates/server/src/x.rs",
            "crates/providers/src/x.rs",
        ] {
            let [finding] = &check(Path::new(path), &text)[..] else {
                panic!("expected one finding for {input} in {path}");
            };
            assert_eq!(finding.line, Some(2));
            assert_eq!(finding.rule, RULE);
            assert!(
                finding.message.contains("input::media"),
                "{}",
                finding.message
            );
        }
    }
}

#[test]
fn ffprobe_in_a_file_that_never_calls_the_helper_is_flagged() {
    let bare = "fn a() {\n    let c = tools.ffprobe().arg(file);\n}\n";
    let [finding] = &check(Path::new(RENDER), bare)[..] else {
        panic!("expected one finding");
    };
    assert_eq!(finding.line, Some(2));
    let helped = "fn a() {\n    let mut c = tools.ffprobe();\n    input::media(&mut c, file);\n}\n";
    assert!(check(Path::new(RENDER), helped).is_empty());
}

#[test]
fn an_input_elsewhere_or_in_a_comment_or_a_longer_literal_passes() {
    let input = "fn a() { command.arg(\"-i\"); }\n";
    assert!(check(Path::new("crates/render/tests/suite/x.rs"), input).is_empty());
    assert!(check(Path::new("crates/golden/src/harness/setup.rs"), input).is_empty());
    let quiet =
        "//! Once `\"-i\"` was bare here.\nfn a() { c.args([\"-in\", \"x -i y\"]); // \"-i\"\n}\n";
    assert!(check(Path::new(RENDER), quiet).is_empty());
}

#[test]
fn an_allow_with_a_reason_excuses_the_line_below_it() {
    let text = format!("// lint: allow({RULE}) a lavfi graph, not a file\nc.args([\"-i\", g]);\n");
    assert!(check(Path::new(RENDER), &text).is_empty());
    let text = format!("// lint: allow({RULE})\nc.args([\"-i\", g]);\n");
    assert_eq!(check(Path::new(RENDER), &text).len(), 1);
}
