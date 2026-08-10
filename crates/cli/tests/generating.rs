//! `scorsese generate` and the question it asks before it spends.
//!
//! Every test here runs the real binary with stdin closed — which is what a
//! pipe, a CI job and an agent all look like — and asserts the effect on the
//! document: a shot that was never submitted still says `sketch` and carries no
//! operation. Nothing reaches a provider, and nothing here could: the runs are
//! pointed at a settings file of our own holding a key that is not a key, and
//! every one of them stops before the call that would use it.

#[path = "common/mod.rs"]
mod common;

use std::path::{Path, PathBuf};

use common::{documents, reload, run_in_env, temp_dir};

/// A project with one shot nobody has paid for.
fn sketched(label: &str) -> PathBuf {
    documents::project(
        label,
        &[documents::assets::sketch("shot")],
        &[documents::clip("c1", "shot", 0)],
    )
}

/// A machine of our own: a settings file with `ceiling` cents on it, holding
/// keys that are not keys.
///
/// Never the tester's own settings, which hold a real key and a real ceiling —
/// a test that read those would be one provider change away from spending
/// somebody's money.
fn machine(label: &str, ceiling: u64) -> PathBuf {
    let home = temp_dir(label);
    let settings = format!(
        r#"{{ "gemini_api_key": "not-a-key", "elevenlabs_api_key": "not-a-key",
              "budget_cents": {ceiling} }}"#
    );
    // Both places a settings file lives, because which one is read is decided
    // by the platform and this test does not care to know which.
    for relative in ["scorsese", "Library/Application Support/scorsese"] {
        let dir = home.join(relative);
        std::fs::create_dir_all(&dir).expect("create the settings directory");
        std::fs::write(dir.join("settings.json"), &settings).expect("write settings.json");
    }
    home
}

/// The environment that points a run at that machine instead of this one.
fn at(home: &Path) -> [(&'static str, &Path); 3] {
    [("XDG_CONFIG_HOME", home), ("APPDATA", home), ("HOME", home)]
}

/// The document as it stands, which is where "nothing was submitted" is read.
fn document(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("project.json")).expect("read project.json")
}

#[track_caller]
fn nothing_was_submitted(dir: &Path) {
    let document = document(dir);
    assert!(
        !document.contains("operation") && document.contains("sketch"),
        "the shot was submitted:\n{document}"
    );
}

#[test]
fn a_pipe_is_refused_and_told_how_to_run_unattended() {
    let dir = sketched("generate-piped");
    let home = machine("generate-piped-home", 0);

    let run = run_in_env(&dir, &["generate"], &at(&home));

    assert!(run.failed, "the run spent without asking:\n{}", run.output);
    run.says("--yes");
    nothing_was_submitted(&dir);
}

#[test]
fn the_quote_is_still_free_and_answers_to_nobody() {
    let dir = sketched("generate-quoted");
    let home = machine("generate-quoted-home", 0);

    let run = run_in_env(&dir, &["generate", "--dry-run"], &at(&home)).ok();

    run.says("for the whole run");
    nothing_was_submitted(&dir);
}

/// The one that pins where the ceiling is checked.
///
/// `--yes` says nobody is there to be asked, which is exactly the situation the
/// ceiling exists for — so the check has to live below the flag, inside the
/// pass that submits, and not beside it. A refactor that moved it up next to
/// `--yes` would turn the flag into a way past the ceiling, and would fail
/// here.
#[test]
fn the_ceiling_is_refused_even_under_yes() {
    let dir = sketched("generate-over-ceiling");
    let home = machine("generate-over-ceiling-home", 0);

    let run = run_in_env(&dir, &["generate", "--yes"], &at(&home));

    assert!(run.failed, "the ceiling let it through:\n{}", run.output);
    run.says("ceiling");
    nothing_was_submitted(&dir);
}

#[test]
fn a_project_with_nothing_to_send_is_never_asked() {
    let dir = documents::project(
        "generate-nothing-to-do",
        &[documents::assets::generated("shot", "video", "mp4")],
        &[documents::clip("c1", "shot", 0)],
    );
    // The file that answers "anything to do?" is the *current brief's* output
    // in generated/, named for the brief's hash — never the recorded path.
    documents::file_at(&dir, &brief_output(&dir, "shot"), b"");
    let home = machine("generate-nothing-to-do-home", 0);

    // No question, no refusal, no key: a run that would spend nothing has
    // nothing to confirm, and a script that runs this twice must not break the
    // second time.
    run_in_env(&dir, &["generate"], &at(&home)).ok();
}

/// Where the current brief of `id` would land — the file the run checks for,
/// computed the way the run computes it.
fn brief_output(dir: &Path, id: &str) -> String {
    let project = reload(dir);
    let asset = project
        .assets
        .iter()
        .find(|asset| asset.id.to_string() == id)
        .expect("the asset is in the table");
    scorsese_providers::video::Brief::of(&project, dir, asset)
        .expect("gather the brief")
        .output()
        .as_str()
        .to_owned()
}

/// The bug that was hit in real use: a stale shot whose *old* generation is
/// still on disk at the recorded path. The run used to consult that path,
/// find the file, print "No generated assets in this project." and submit
/// nothing. The ceiling refusing it here proves the run now wants to send the
/// edited brief — the refusal is checked inside the pass, before any call.
#[test]
fn a_stale_shot_with_its_old_file_present_is_still_sent() {
    let dir = documents::project(
        "generate-stale",
        &[documents::assets::stale("shot")],
        &[documents::clip("c1", "shot", 0)],
    );
    documents::file_at(&dir, "generated/shot.mp4", b"the old take");
    let home = machine("generate-stale-home", 0);

    let run = run_in_env(&dir, &["generate", "--yes"], &at(&home));

    assert!(
        run.failed,
        "the old file hid the stale shot:\n{}",
        run.output
    );
    run.says("ceiling");
}

/// The quote prices what THIS run would send. A brief whose output is already
/// in generated/ is listed at nothing — it used to be priced like a sketch,
/// quoting money the run could never spend.
#[test]
fn the_quote_charges_nothing_for_a_brief_already_generated() {
    let dir = sketched("generate-quote-cached");
    documents::file_at(&dir, &brief_output(&dir, "shot"), b"the take");
    let home = machine("generate-quote-cached-home", 0);

    let run = run_in_env(&dir, &["generate", "--dry-run"], &at(&home)).ok();

    run.says("already generated");
    run.says("$0.00 for the whole run");
}
