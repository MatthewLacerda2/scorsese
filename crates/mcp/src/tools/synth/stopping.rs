//! `synth_bake` under a cancel the client already sent (#661): refused as
//! stopped, with nothing written into `generated/`.
//!
//! A unit test rather than a protocol one, because over the wire a cancel
//! races the bake it names, and a bake slow enough to lose that race reliably
//! is the long bake a test must not be. Here the cancel is tripped before the
//! call, so where it lands is not a question.

use serde_json::json;

use super::super::Tool;
use super::Bake;
use scorsese_render::Cancel;

#[test]
fn a_cancelled_bake_stops_and_leaves_generated_empty() {
    let dir = std::env::temp_dir().join(format!("scorsese-bake-stopped-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    let project = json!({ "project": dir });
    for (tool, arguments) in [
        ("project_new", project.clone()),
        (
            "synth_new",
            json!({ "project": dir, "name": "bed", "kind": "song" }),
        ),
    ] {
        let tool = super::super::find(tool).expect("a registered tool");
        tool.call(&arguments).expect("the fixture is made");
    }

    let cancel = Cancel::new();
    cancel.cancel();
    let Err(refused) = Bake.call_cancellable(&project, &cancel) else {
        panic!("a cancelled bake is not answered with a bake");
    };
    assert!(refused.contains("stopped"), "{refused}");
    let generated = std::fs::read_dir(dir.join("generated")).map_or(0, Iterator::count);
    assert_eq!(generated, 0, "nothing half-made at a bake's address");

    // And the same call, not cancelled, bakes — so the refusal above was the
    // cancel's and not a broken fixture's.
    Bake.call(&project).expect("bakes");
    std::fs::remove_dir_all(dir).ok();
}
