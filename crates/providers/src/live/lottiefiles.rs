//! LottieFiles' part of the live check: one search, free and keyless (#910).
//!
//! **`searchPublicAnimations(query: "rocket", first: 3)`** — that the endpoint
//! still answers anonymously, and the shape
//! [`LottieLibrary`](crate::stock::LottieLibrary) reads: results, each with an
//! id, a `jsonUrl` and an `imageUrl`, and `metadata` with a width, a height, a
//! frame rate and a duration.
//!
//! Two failures matter more than the rest. A field renamed would leave an
//! agent's searches empty with nothing saying why; a `401`/`403` (or a GraphQL
//! error that talks about authorisation, answered as `200`) means anonymous
//! access was withdrawn, and #903 names the fallback for that day: the
//! maintainer's LottieFiles account. The verdict says so, so whoever reads it
//! knows which of the two to fix.

use crate::api::http::HttpError;
use crate::api::lottiefiles::{Connection, ENDPOINT, LottieFiles, Metadata};
use crate::api::tap::Tap;

use super::{Step, Verdict, judge};

/// What the search looks for: something the library is certain to have.
pub const WORDS: &str = "rocket";

/// How many results it asks for.
pub const FIRST: u32 = 3;

/// The call, as a person would name it.
const CALL: &str = "POST searchPublicAnimations";

/// The calls, as a quote lists them.
pub(super) fn calls() -> Vec<String> {
    vec![format!(
        "POST {ENDPOINT} searchPublicAnimations(\"{WORDS}\", first: {FIRST}) — free, no key"
    )]
}

/// Runs the LottieFiles part; it spends nothing.
pub(super) fn check(tap: &Tap) -> (Vec<Step>, u64) {
    let answer = LottieFiles::new().tapped(tap).search(WORDS, FIRST);
    (vec![search_step(answer)], 0)
}

/// Reads one measurement off an animation's metadata.
type Measure = fn(&Metadata) -> Option<f64>;

/// The measurements the library reads, by the name the API gives them.
const MEASURED: [(&str, Measure); 4] = [
    ("width", |m| m.width),
    ("height", |m| m.height),
    ("frameRate", |m| m.frame_rate),
    ("duration", |m| m.duration),
];

/// What the search gave back.
///
/// Every result must carry its JSON and its still: a result without them is
/// one the library drops or shows blank. A measurement is held to a looser
/// bar — **at least one** result carries it — because an old upload may lack
/// `metadata` and that is not the API changing; every result missing it is.
pub fn search_step(answer: Result<Connection, HttpError>) -> Step {
    let found = match answer {
        Err(error) => return Step::new(CALL, judge::lottiefiles(&error)),
        Ok(found) => found,
    };
    let nodes: Vec<_> = found.edges.iter().map(|edge| &edge.node).collect();
    let given = |url: &Option<String>| url.as_deref().is_some_and(|url| !url.is_empty());
    let field = if nodes.is_empty() {
        Some(String::from("edges: the search came back empty"))
    } else if let Some(node) = nodes.iter().find(|node| !given(&node.json_url)) {
        Some(format!("edges[].node.jsonUrl: result {} had none", node.id))
    } else if let Some(node) = nodes.iter().find(|node| !given(&node.image_url)) {
        Some(format!(
            "edges[].node.imageUrl: result {} had none",
            node.id
        ))
    } else {
        MEASURED.iter().find_map(|(name, read)| {
            let carried = nodes
                .iter()
                .any(|node| node.metadata.as_ref().and_then(read).is_some());
            (!carried).then(|| {
                format!(
                    "edges[].node.metadata.{name}: none of {} results carried it",
                    nodes.len()
                )
            })
        })
    };
    match field {
        Some(field) => Step::new(CALL, Verdict::ShapeChanged { field }),
        None => Step::new(CALL, Verdict::Ok).noting(format!(
            "{} results of {}",
            nodes.len(),
            found.total_count
        )),
    }
}
