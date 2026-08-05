//! What happens when the provider takes a brief and then says no.
//!
//! Split from [`lifecycle`](super::lifecycle) because a refusal is the other
//! half of the state machine, and the half with the money in it: the run has
//! already been paid for by the time one of twenty shots comes back rejected,
//! so what the other nineteen do is not a detail.

use scorsese_providers::credentials::Budget;
use scorsese_providers::video::{Outcome, generate};

use crate::mock::{Answer, Mock};
use crate::{sketched, standing};

/// A refused brief goes back to being a sketch: it is a prompt to edit and try
/// again, and an asset still claiming to be queued would be polled for ever.
#[test]
fn a_refused_brief_goes_back_to_being_a_sketch() {
    let (dir, mut project, id) = sketched("refused", "something the model will not make");
    let provider = Mock::answering(
        "shot",
        vec![Answer::Failed(String::from("the prompt was rejected"))],
    );

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let second = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let Outcome::Failed { message } = &second[0].1 else {
        panic!("expected a refusal, got {:?}", second[0].1);
    };
    assert!(message.contains("rejected"), "{message}");

    let (state, path, ticket) = standing(&project, &id);
    assert_eq!(state, "Some(Sketch)");
    assert_eq!(path, None);
    assert_eq!(ticket, None, "a dead ticket is not worth polling");
    std::fs::remove_dir_all(&dir).ok();
}

/// The failure that actually happens at three in the morning: several shots,
/// one of them refused. The others must be finished and said to be.
#[test]
fn one_shot_failing_leaves_the_rest_generated() {
    let (dir, mut project) = crate::common::project("partial");
    for (id, prompt) in [("a", "a wide shot"), ("b", "a close-up"), ("c", "a pan")] {
        project.assets.push(scorsese_core::Asset::sketch(
            scorsese_core::AssetId::new(id),
            scorsese_core::AssetKind::GeneratedVideo,
            prompt,
        ));
    }
    let provider = Mock::answering("a", vec![Answer::Ready(b"AAA".to_vec())])
        .and("b", vec![Answer::Failed(String::from("no"))])
        .and("c", vec![Answer::Ready(b"CCC".to_vec())]);

    generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");
    let second = generate(&mut project, &dir, &provider, Budget::unlimited(0)).expect("a run");

    let by_id = |want: &str| {
        second
            .iter()
            .find(|(id, _)| id.as_str() == want)
            .map(|(_, outcome)| outcome)
            .expect("every asset is reported on")
    };
    assert!(matches!(by_id("a"), Outcome::Generated { .. }));
    assert!(matches!(by_id("b"), Outcome::Failed { .. }));
    assert!(matches!(by_id("c"), Outcome::Generated { .. }));
    std::fs::remove_dir_all(&dir).ok();
}
