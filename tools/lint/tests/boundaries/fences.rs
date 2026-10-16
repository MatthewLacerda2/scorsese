//! Words a part of the tree may not write.

use std::path::Path;

use scorsese_lint::rules::boundary::fences::check;

fn rules(path: &str, text: &str) -> Vec<&'static str> {
    check(Path::new(path), text)
        .iter()
        .map(|finding| finding.rule)
        .collect()
}

#[test]
fn zimmer_doing_io_is_flagged_and_its_doc_saying_so_is_not() {
    let found = check(
        Path::new("crates/zimmer/src/a.rs"),
        "fn a() {}\n\nstd::fs::read(p);\n",
    );
    assert_eq!(found[0].line, Some(3));
    assert_eq!(
        rules("crates/zimmer/src/bake.rs", "let f = std::fs::read(p);\n"),
        ["no-io"]
    );
    assert_eq!(
        rules("crates/zimmer/src/bake.rs", "    println!(\"{x}\");\n"),
        ["no-io"]
    );
    assert!(rules("crates/zimmer/src/lib.rs", "//! no std::fs, no println!\n").is_empty());
    assert!(rules("crates/zimmer/tests/bake.rs", "std::fs::write(p, b)?;\n").is_empty());
    assert!(rules("crates/providers/src/synth.rs", "std::fs::write(p, b)?;\n").is_empty());
}

#[test]
fn a_word_matches_only_whole() {
    assert!(rules("crates/zimmer/src/a.rs", "let x = std::fsx::y;\n").is_empty());
    assert_eq!(
        rules("crates/zimmer/src/a.rs", "eprint!(\"x\");\n"),
        ["no-io"]
    );
    assert!(rules("crates/core/src/a.rs", "let c = my_process::Commander;\n").is_empty());
    assert_eq!(
        rules("crates/core/src/a.rs", "use std::process::Command;\n"),
        ["no-io"]
    );
    assert_eq!(
        rules("crates/guide/src/a.rs", "std::env::args()\n"),
        ["no-io"]
    );
}

#[test]
fn render_may_read_zimmer_but_not_synthesise_with_it() {
    let path = "crates/render/src/audio/mix.rs";
    assert!(rules(path, "use scorsese_zimmer::level::Meter;\n").is_empty());
    assert!(rules(path, "use scorsese_zimmer::survey::Survey;\n").is_empty());
    assert_eq!(
        rules(path, "use scorsese_zimmer::bake_song;\n"),
        ["zimmer-reads"]
    );
    assert_eq!(
        rules(path, "use scorsese_zimmer::levels;\n"),
        ["zimmer-reads"]
    );
    assert_eq!(
        rules(path, "use scorsese_zimmer::{level, bake};\n"),
        ["zimmer-reads"]
    );
}

#[test]
fn ffmpeg_is_spawned_only_by_the_command_builder() {
    let spawn = "let c = Command::new(\"ffmpeg\");\n";
    assert_eq!(rules("crates/cli/src/a.rs", spawn), ["ffmpeg"]);
    assert_eq!(
        rules("app/src/a.rs", "Command::new(\"ffprobe\")\n"),
        ["ffmpeg"]
    );
    assert!(rules("crates/render/src/tools.rs", spawn).is_empty());
    assert!(rules("crates/server/src/a.rs", "Command::new(\"docker\")\n").is_empty());
}

#[test]
fn one_module_of_providers_names_an_http_library() {
    let line = "let agent = ureq::Agent::new_with_defaults();\n";
    assert!(rules("crates/providers/src/api/http/mod.rs", line).is_empty());
    assert_eq!(
        rules("crates/providers/src/api/veo/mod.rs", line),
        ["owner"]
    );
    assert!(rules("crates/providers/src/a.rs", "let pureq = 1;\n").is_empty());
}

#[test]
fn an_allow_with_a_reason_excuses_the_line_below_it() {
    let text = "// lint: allow(ffmpeg) reads arguments, never runs\nCommand::new(\"ffmpeg\")\n";
    assert!(rules("crates/cli/src/a.rs", text).is_empty());
    let text = "// lint: allow(ffmpeg)\nCommand::new(\"ffmpeg\")\n";
    assert_eq!(rules("crates/cli/src/a.rs", text), ["ffmpeg"]);
}
