//! The sentence each shot outcome reads as — pinned word for word, because the
//! CLI and the MCP server both print it and an agent reads it to learn what
//! happened to a shot.

use scorsese_core::{ProjectPath, Timestamp};
use scorsese_providers::video::{Outcome, RETENTION_DAYS};

#[test]
fn every_shot_outcome_reads_as_its_sentence() {
    let path = || ProjectPath::new("generated/shot-abc.mp4");
    let operation = || String::from("operations/42");
    let cases = [
        (
            Outcome::Cached { path: path() },
            "already generated — generated/shot-abc.mp4",
        ),
        (
            Outcome::Queued {
                operation: operation(),
                estimated_cost_cents: 96,
            },
            "queued — operations/42",
        ),
        (
            Outcome::Waiting {
                operation: operation(),
                queued_at: None,
            },
            "still generating — operations/42",
        ),
        (
            Outcome::Generated {
                path: path(),
                bytes: 1234,
                estimated_cost_cents: 96,
            },
            "generated — generated/shot-abc.mp4 (1234 bytes)",
        ),
        (
            Outcome::Failed {
                message: String::from("no"),
            },
            "refused — no",
        ),
    ];
    for (outcome, sentence) in cases {
        assert_eq!(outcome.says(), sentence);
    }
}

#[test]
fn an_expired_shot_says_when_it_was_queued_and_that_it_is_gone() {
    let expired = |queued_at| Outcome::Expired {
        operation: String::from("operations/42"),
        queued_at,
    };
    let stamp = Timestamp::from_utc(2026, 9, 1, 12, 0, 0).expect("a valid instant");
    assert_eq!(
        expired(Some(stamp)).says(),
        format!(
            "queued 2026-09-01T12:00:00Z and past the {RETENTION_DAYS}-day window — the video \
             is gone and the shot has to be asked for again"
        )
    );
    assert!(
        expired(None)
            .says()
            .starts_with("queued at some point and past the ")
    );
}
