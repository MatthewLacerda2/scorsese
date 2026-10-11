//! Taking turns at a project: a second changer waits, then goes, or is refused
//! by name; another project, or the same one spelled differently, is judged as
//! what it is.

use std::sync::mpsc::channel;
use std::time::Duration;

use super::hold::Holds;

const BRIEF: Duration = Duration::from_millis(50);

#[test]
fn a_second_changer_is_refused_by_the_name_of_the_first_once_its_patience_runs_out() {
    let holds = Holds::default();
    let _first = holds
        .take_within("a.scor", "generate", BRIEF)
        .expect("free");
    let Err(refused) = holds.take_within("./a.scor", "clip_set", BRIEF) else {
        panic!("the same project, spelled differently, was held twice");
    };
    assert!(
        refused.starts_with("`generate` is still changing ./a.scor"),
        "{refused}"
    );
    assert!(refused.contains("nothing was changed"), "{refused}");
    assert!(
        holds.take_within("b.scor", "clip_set", BRIEF).is_ok(),
        "another project waits for nothing"
    );
}

#[test]
fn a_second_changer_goes_as_soon_as_the_first_lets_go() {
    let holds = Holds::default();
    let first = holds
        .take_within("c.scor", "clip_set", BRIEF)
        .expect("free");
    let (went, gone) = channel();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            let second = holds.take_within("c.scor", "clip_move", Duration::from_secs(30));
            went.send(second.is_ok()).expect("the test listens");
        });
        assert!(
            gone.recv_timeout(BRIEF).is_err(),
            "it went while the project was held"
        );
        drop(first);
        assert_eq!(gone.recv_timeout(Duration::from_secs(5)), Ok(true));
    });
}

#[test]
fn a_hold_is_let_go_when_its_call_ends_however_it_ends() {
    let holds = Holds::default();
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _held = holds
            .take_within("d.scor", "asset_set", BRIEF)
            .expect("free");
        panic!("a bug in a tool");
    }));
    assert!(unwound.is_err());
    assert!(holds.take_within("d.scor", "asset_set", BRIEF).is_ok());
}
