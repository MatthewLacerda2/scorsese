//! The sentence each still outcome reads as — pinned word for word, because the
//! CLI and the MCP server both print it.

use scorsese_core::ProjectPath;
use scorsese_providers::image::Outcome;

#[test]
fn every_still_outcome_reads_as_its_sentence() {
    let path = || ProjectPath::new("generated/still-abc.jpg");
    let cases = [
        (
            Outcome::Cached { path: path() },
            "already drawn — generated/still-abc.jpg",
        ),
        (
            Outcome::Generated {
                path: path(),
                bytes: 1234,
                estimated_cost_cents: 2,
            },
            "drawn — generated/still-abc.jpg (1234 bytes)",
        ),
        (
            Outcome::Incomplete {
                why: String::from("A reference still that is still a sketch"),
            },
            "not yet — A reference still that is still a sketch",
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
