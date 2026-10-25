//! The landing page's A/B tests (#1553): who saw which version, and what they
//! did, for the operator's report.
//!
//! **The tests are the web app's**, defined in its code (`web/src/ab/`); this
//! side never knows them, and holds only the shape of what a browser sends.
//! A browser carries a random visitor code in its `scorsese_visitor` cookie
//! ([`VISITOR_COOKIE`]), set by the web the first time the landing loads, and
//! draws each version from a hash of (code, test), so nothing needs storing
//! for it to see the same version again. It sends [`POST /api/ab/events`]
//! (`http::ab`): the versions it was shown ([`Exposure`]) and what happened
//! ([`Event`]). An event belongs to the visitor, not to a test, so it counts
//! in every test the visitor is in.
//!
//! **Tied to the account at sign-in**: a Google sign-in whose browser carries
//! the cookie links that visitor to the account ([`link`]), which is how the
//! report relates the versions a person saw to whether they paid. Deleting the
//! account sets the link back to NULL; the counts stay.
//!
//! The route is anonymous and public, so it is bounded twice over: a batch is
//! small ([`Batch::valid`]), and a visitor keeps at most
//! [`EVENTS_PER_VISITOR_PER_DAY`] events and [`TESTS_PER_VISITOR`] exposures.
//! What goes over is dropped without a word. A failed write is logged and
//! never answered as an error: nothing a visitor does depends on it.
//!
//! The tables are migration `0048`'s; [`report`] reads them, and [`decide`]
//! is the rule a test is decided by.

pub mod decide;
pub mod domains;
pub mod report;

use serde::Deserialize;
use sqlx::postgres::PgPool;

use crate::db::{self, UserId};

/// The cookie the web app keeps the visitor code in.
pub const VISITOR_COOKIE: &str = "scorsese_visitor";

/// Versions one batch may name.
pub const EXPOSURES_PER_BATCH: usize = 8;

/// Events one batch may carry.
pub const EVENTS_PER_BATCH: usize = 16;

/// Events one visitor keeps over a day; the rest are dropped.
pub const EVENTS_PER_VISITOR_PER_DAY: i64 = 200;

/// Tests one visitor may be in, ever.
pub const TESTS_PER_VISITOR: i64 = 32;

/// The most visible seconds one `time` event may carry: half an hour.
pub const MOST_SECONDS: i32 = 30 * 60;

/// What a browser sends: who it is, what it was shown, what it did.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Batch {
    /// The visitor code, a UUID.
    pub visitor: String,
    /// The versions it is being shown.
    #[serde(default)]
    pub exposures: Vec<Exposure>,
    /// What happened.
    #[serde(default)]
    pub events: Vec<Event>,
}

/// One test's version, as shown.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exposure {
    /// The test's id.
    pub test: String,
    /// The version's id.
    pub version: String,
}

/// One thing a visitor did.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    /// What.
    pub kind: Kind,
    /// The deepest section for `section`, seconds for `time`, else 0.
    #[serde(default)]
    pub value: i32,
}

/// The kinds of [`Event`], as the `kind` column writes them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// The landing was shown.
    View,
    /// A section was reached: the deepest so far, counted from 1.
    Section,
    /// The last section was reached.
    End,
    /// Seconds the page was visible, sent when it was hidden.
    Time,
    /// The prompt box was sent.
    Prompt,
}

impl Kind {
    fn column(self) -> &'static str {
        match self {
            Self::View => "view",
            Self::Section => "section",
            Self::End => "end",
            Self::Time => "time",
            Self::Prompt => "prompt",
        }
    }

    fn takes(self, value: i32) -> bool {
        match self {
            Self::Section => (1..=64).contains(&value),
            Self::Time => (0..=MOST_SECONDS).contains(&value),
            Self::View | Self::End | Self::Prompt => value == 0,
        }
    }
}

/// Where Cloudflare placed a visitor, when the request came through it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Place {
    /// The ISO 3166 country code, e.g. `BR`.
    pub country: Option<String>,
    /// The country's subdivision code, e.g. `SP`.
    pub region: Option<String>,
}

impl Place {
    /// The place headers say, each kept only if it has a code's shape.
    pub fn new(country: Option<&str>, region: Option<&str>) -> Self {
        fn code(value: Option<&str>, lengths: std::ops::RangeInclusive<usize>) -> Option<String> {
            let value = value?.trim();
            let shaped = lengths.contains(&value.len())
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit());
            shaped.then(|| value.to_owned())
        }
        Self {
            country: code(country, 2..=2),
            region: code(region, 1..=3),
        }
    }
}

impl Batch {
    /// Whether the batch has the shape a client of ours sends.
    pub fn valid(&self) -> bool {
        let id = |id: &str| {
            (1..=32).contains(&id.len())
                && id
                    .bytes()
                    .all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'-'))
        };
        visitor_code(&self.visitor).is_some()
            && self.exposures.len() <= EXPOSURES_PER_BATCH
            && self.events.len() <= EVENTS_PER_BATCH
            && self
                .exposures
                .iter()
                .all(|seen| id(&seen.test) && id(&seen.version))
            && self
                .events
                .iter()
                .all(|event| event.kind.takes(event.value))
    }

    /// Rows the batch would write, for the per-address count.
    pub fn rows(&self) -> usize {
        1 + self.exposures.len() + self.events.len()
    }
}

/// `value` as a visitor code: a UUID, lowercased.
pub fn visitor_code(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let shaped = bytes.len() == 36
        && bytes.iter().enumerate().all(|(at, byte)| match at {
            8 | 13 | 18 | 23 => *byte == b'-',
            _ => byte.is_ascii_hexdigit(),
        });
    shaped.then(|| value.to_ascii_lowercase())
}

/// Write `batch`, already [`Batch::valid`], from a visitor at `place`.
///
/// The visitor is created on its first batch, with the place it came from.
/// A version already recorded for a test stays the one recorded.
pub async fn record(pool: &PgPool, batch: &Batch, place: &Place) -> Result<(), sqlx::Error> {
    let Some(code) = visitor_code(&batch.visitor) else {
        return Ok(());
    };
    let mut tx = db::privileged(pool).await?;
    sqlx::query(
        "INSERT INTO ab_visitors (code, country, region) VALUES ($1::uuid, $2, $3)
         ON CONFLICT (code) DO NOTHING",
    )
    .bind(&code)
    .bind(&place.country)
    .bind(&place.region)
    .execute(&mut *tx)
    .await?;
    // Serialises one visitor's batches, so the counts below hold.
    sqlx::query("SELECT 1 FROM ab_visitors WHERE code = $1::uuid FOR UPDATE")
        .bind(&code)
        .execute(&mut *tx)
        .await?;
    let (tests, today): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM ab_exposures WHERE visitor = $1::uuid),
                (SELECT count(*) FROM ab_events
                 WHERE visitor = $1::uuid AND at > now() - interval '1 day')",
    )
    .bind(&code)
    .fetch_one(&mut *tx)
    .await?;
    let room = usize::try_from(TESTS_PER_VISITOR - tests).unwrap_or(0);
    for seen in batch.exposures.iter().take(room) {
        sqlx::query(
            "INSERT INTO ab_exposures (visitor, test, version) VALUES ($1::uuid, $2, $3)
             ON CONFLICT (visitor, test) DO NOTHING",
        )
        .bind(&code)
        .bind(&seen.test)
        .bind(&seen.version)
        .execute(&mut *tx)
        .await?;
    }
    let room = usize::try_from(EVENTS_PER_VISITOR_PER_DAY - today).unwrap_or(0);
    for event in batch.events.iter().take(room) {
        sqlx::query("INSERT INTO ab_events (visitor, kind, value) VALUES ($1::uuid, $2, $3)")
            .bind(&code)
            .bind(event.kind.column())
            .bind(event.value)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await
}

/// Link the visitor `code` names to `user`, who has just signed in.
///
/// A visitor already linked keeps its first account; an unknown code links
/// nothing. A failure is logged, never the sign-in's.
pub async fn link(pool: &PgPool, code: &str, user: UserId) {
    let Some(code) = visitor_code(code) else {
        return;
    };
    let linked = async {
        let mut tx = db::privileged(pool).await?;
        sqlx::query(
            "UPDATE ab_visitors SET user_id = $2, linked_at = now()
             WHERE code = $1::uuid AND user_id IS NULL",
        )
        .bind(&code)
        .bind(user.get())
        .execute(&mut *tx)
        .await?;
        tx.commit().await
    };
    if let Err(error) = linked.await {
        eprintln!(
            "scorsese-server: could not link a visitor to account {}: {error}",
            user.get()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn batch(visitor: &str, exposures: &[(&str, &str)], events: &[(Kind, i32)]) -> Batch {
        Batch {
            visitor: visitor.into(),
            exposures: exposures
                .iter()
                .map(|(test, version)| Exposure {
                    test: (*test).into(),
                    version: (*version).into(),
                })
                .collect(),
            events: events
                .iter()
                .map(|&(kind, value)| Event { kind, value })
                .collect(),
        }
    }

    const CODE: &str = "0b7f6a3e-9c1d-4e2f-8a5b-6c7d8e9f0a1b";

    #[test]
    fn a_visitor_code_is_a_uuid() {
        assert_eq!(
            visitor_code("0B7F6A3E-9C1D-4E2F-8A5B-6C7D8E9F0A1B").as_deref(),
            Some(CODE)
        );
        for not in [
            "",
            "0b7f6a3e9c1d4e2f8a5b6c7d8e9f0a1b",
            "0b7f6a3e-9c1d-4e2f-8a5b-6c7d8e9f0a1g",
        ] {
            assert_eq!(visitor_code(not), None, "{not}");
        }
    }

    #[test]
    fn a_batch_of_our_shape_is_valid() {
        let ok = batch(
            CODE,
            &[("heading", "b")],
            &[
                (Kind::View, 0),
                (Kind::Section, 3),
                (Kind::Time, 1800),
                (Kind::Prompt, 0),
            ],
        );
        assert!(ok.valid());
        assert_eq!(ok.rows(), 6);
    }

    #[test]
    fn anything_else_is_not() {
        let refused = [
            batch("me", &[], &[]),
            batch(CODE, &[("Heading", "b")], &[]),
            batch(CODE, &[("heading", "b; drop")], &[]),
            batch(CODE, &[(&"x".repeat(33), "b")], &[]),
            batch(CODE, &[], &[(Kind::Section, 0)]),
            batch(CODE, &[], &[(Kind::Time, 1801)]),
            batch(CODE, &[], &[(Kind::Prompt, 1)]),
            batch(CODE, &[("a", "b"); EXPOSURES_PER_BATCH + 1], &[]),
            batch(CODE, &[], &[(Kind::View, 0); EVENTS_PER_BATCH + 1]),
        ];
        for batch in refused {
            assert!(!batch.valid(), "{batch:?}");
        }
    }

    #[test]
    fn a_place_keeps_only_codes() {
        assert_eq!(
            Place::new(Some("BR"), Some("SP")),
            Place {
                country: Some("BR".into()),
                region: Some("SP".into())
            }
        );
        assert_eq!(
            Place::new(Some("Brazil"), Some("São Paulo")),
            Place::default()
        );
        assert_eq!(Place::new(Some(""), Some("ABCD")), Place::default());
        assert_eq!(Place::new(None, Some("13")).region.as_deref(), Some("13"));
    }
}
