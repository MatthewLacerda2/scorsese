//! `scorsese-server ab report`: the landing's tests as the operator decides
//! them (#1553, docs/web.md *A/B tests*).
//!
//! For each test, each version: visitors, the share who reached the last
//! section, the median visible seconds, the share who sent a prompt, signed
//! in and paid, and the share of company accounts among those signed in
//! ([`super::domains`]). Then each version's chance of being the best on the
//! prompt rate and on reaching the last section, the verdict of
//! [`super::decide`], and the same split by hour band in Brasília time and by
//! region, a cell under [`THIN`] visitors marked thin.
//!
//! Only what happened from a visitor's exposure on counts towards a test.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use clap::Subcommand;
use sqlx::postgres::PgPool;

use super::decide::{self, Arm, Signal, Verdict};
use super::domains;
use crate::ServerError;
use crate::db;

/// A split's cell with fewer visitors than this is marked thin.
pub const THIN: usize = 20;

/// `scorsese-server ab …`
#[derive(Debug, Subcommand)]
pub enum AbCommand {
    /// Print every landing test's versions, what their visitors did, and
    /// whether the test is ready to decide.
    Report {
        /// Only this test.
        #[arg(long)]
        test: Option<String>,
    },
}

/// One visitor in one test, as the report counts them.
#[derive(Debug, Clone, sqlx::FromRow)]
struct Seen {
    test: String,
    version: String,
    /// The exposure's hour in Brasília.
    hour: i32,
    place: Option<String>,
    ended: bool,
    prompted: bool,
    seconds: i64,
    email: Option<String>,
    paid: bool,
}

/// Carry out `command`, returning what to print.
pub async fn run(pool: &PgPool, command: AbCommand) -> Result<String, ServerError> {
    let AbCommand::Report { test } = command;
    let mut tx = db::privileged(pool).await.map_err(ServerError::Database)?;
    let seen: Vec<Seen> = sqlx::query_as(
        "SELECT x.test, x.version,
                extract(hour FROM x.at AT TIME ZONE 'America/Sao_Paulo')::int AS hour,
                v.country || coalesce('-' || v.region, '') AS place,
                EXISTS (SELECT 1 FROM ab_events e WHERE e.visitor = x.visitor
                        AND e.kind = 'end' AND e.at >= x.at) AS ended,
                EXISTS (SELECT 1 FROM ab_events e WHERE e.visitor = x.visitor
                        AND e.kind = 'prompt' AND e.at >= x.at) AS prompted,
                coalesce((SELECT sum(e.value) FROM ab_events e WHERE e.visitor = x.visitor
                          AND e.kind = 'time' AND e.at >= x.at), 0)::bigint AS seconds,
                u.email,
                EXISTS (SELECT 1 FROM payments p WHERE p.user_id = u.id
                        AND p.state = 'paid') AS paid
         FROM ab_exposures x
         JOIN ab_visitors v ON v.code = x.visitor
         LEFT JOIN users u ON u.id = v.user_id
         WHERE $1::text IS NULL OR x.test = $1
         ORDER BY x.test, x.version",
    )
    .bind(&test)
    .fetch_all(&mut *tx)
    .await
    .map_err(ServerError::Database)?;
    tx.commit().await.map_err(ServerError::Database)?;
    Ok(render(&seen))
}

/// The report of `seen`, every test in it.
fn render(seen: &[Seen]) -> String {
    let mut tests: BTreeMap<&str, BTreeMap<&str, Vec<&Seen>>> = BTreeMap::new();
    for one in seen {
        tests
            .entry(&one.test)
            .or_default()
            .entry(&one.version)
            .or_default()
            .push(one);
    }
    if tests.is_empty() {
        return "no landing test has had a visitor yet\n".into();
    }
    let mut out = String::new();
    for (test, versions) in &tests {
        test_section(&mut out, test, versions);
    }
    out
}

fn test_section(out: &mut String, test: &str, versions: &BTreeMap<&str, Vec<&Seen>>) {
    let _ = writeln!(out, "test {test}");
    let _ = writeln!(
        out,
        "  {:<12} {:>8} {:>12} {:>8} {:>7} {:>9} {:>6} {:>8}",
        "version", "visitors", "last section", "seconds", "prompt", "signed in", "paid", "company"
    );
    for (version, seen) in versions {
        let signed: Vec<&str> = seen.iter().filter_map(|one| one.email.as_deref()).collect();
        let company = signed
            .iter()
            .filter(|email| domains::is_company(email))
            .count();
        let _ = writeln!(
            out,
            "  {:<12} {:>8} {:>12} {:>8} {:>7} {:>9} {:>6} {:>8}",
            version,
            seen.len(),
            share(count(seen, |one| one.ended), seen.len()),
            median(seen.iter().map(|one| one.seconds).collect()),
            share(count(seen, |one| one.prompted), seen.len()),
            share(signed.len(), seen.len()),
            share(count(seen, |one| one.paid), seen.len()),
            share(company, signed.len()),
        );
    }
    let arms = arms(
        versions
            .iter()
            .map(|(version, seen)| (*version, seen.as_slice())),
    );
    for (name, hits) in [
        ("prompt", (|arm: &Arm| arm.prompts) as fn(&Arm) -> u64),
        ("last section", |arm: &Arm| arm.ends),
    ] {
        let counts: Vec<_> = arms.iter().map(|arm| (hits(arm), arm.visitors)).collect();
        let chances: Vec<String> = arms
            .iter()
            .zip(decide::chance_best(&counts))
            .map(|(arm, chance)| format!("{} {:.0}%", arm.version, chance * 100.0))
            .collect();
        let _ = writeln!(
            out,
            "  chance of being best on {name}: {}",
            chances.join(", ")
        );
    }
    let _ = writeln!(out, "  verdict: {}", verdict(&decide::verdict(&arms)));
    split(out, "by hour (Brasília)", versions, |one| {
        if (7..18).contains(&one.hour) {
            "07:00-18:00"
        } else {
            "18:00-07:00"
        }
        .to_owned()
    });
    split(out, "by region", versions, |one| {
        one.place.clone().unwrap_or_else(|| "unknown".into())
    });
}

/// Each version's counts, for the rule.
fn arms<'a>(versions: impl Iterator<Item = (&'a str, &'a [&'a Seen])>) -> Vec<Arm> {
    versions
        .map(|(version, seen)| Arm {
            version: version.to_owned(),
            visitors: seen.len() as u64,
            prompts: count(seen, |one| one.prompted) as u64,
            ends: count(seen, |one| one.ended) as u64,
        })
        .collect()
}

/// The verdict in the operator's words.
fn verdict(verdict: &Verdict) -> String {
    match verdict {
        Verdict::NotReady { more } => format!(
            "not ready: every version needs {more} more visitor{}",
            if *more == 1 { "" } else { "s" }
        ),
        Verdict::Winner {
            version,
            on,
            chance,
        } => format!(
            "ready to decide: {version} wins on {}, {:.0}% sure",
            match on {
                Signal::Prompt => "prompt rate",
                Signal::LastSection => "reaching the last section",
            },
            chance * 100.0
        ),
        Verdict::NoClearDifference => format!(
            "ready to decide: no clear difference at {} per version; keep the preferred version (the first by default) and start the next test",
            decide::CEILING
        ),
    }
}

/// The versions' visitors, prompt rate and last-section rate in each group
/// `group` sorts them into.
fn split(
    out: &mut String,
    title: &str,
    versions: &BTreeMap<&str, Vec<&Seen>>,
    group: impl Fn(&Seen) -> String,
) {
    let _ = writeln!(out, "  {title}");
    let mut cells: BTreeMap<String, BTreeMap<&str, Vec<&Seen>>> = BTreeMap::new();
    for (version, seen) in versions {
        for one in seen {
            cells
                .entry(group(one))
                .or_default()
                .entry(version)
                .or_default()
                .push(one);
        }
    }
    for (name, versions) in &cells {
        let line: Vec<String> = versions
            .iter()
            .map(|(version, seen)| {
                format!(
                    "{version} {} visitors, prompt {}, last section {}{}",
                    seen.len(),
                    share(count(seen, |one| one.prompted), seen.len()),
                    share(count(seen, |one| one.ended), seen.len()),
                    if seen.len() < THIN { " (thin)" } else { "" }
                )
            })
            .collect();
        let _ = writeln!(out, "    {name:<12} {}", line.join("; "));
    }
}

fn count(seen: &[&Seen], what: impl Fn(&Seen) -> bool) -> usize {
    seen.iter().filter(|one| what(one)).count()
}

/// `part` of `whole` as a whole percentage, or a dash when there is no whole.
fn share(part: usize, whole: usize) -> String {
    if whole == 0 {
        return "-".into();
    }
    format!("{:.0}%", part as f64 * 100.0 / whole as f64)
}

/// The median of `values`, the lower of the middle two for an even count.
fn median(mut values: Vec<i64>) -> String {
    values.sort_unstable();
    values
        .get(values.len().saturating_sub(1) / 2)
        .map_or_else(|| "-".into(), i64::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seen(version: &str, hour: i32, prompted: bool, email: Option<&str>) -> Seen {
        Seen {
            test: "heading".into(),
            version: version.into(),
            hour,
            place: Some("BR-SP".into()),
            ended: true,
            prompted,
            seconds: 30,
            email: email.map(Into::into),
            paid: false,
        }
    }

    #[test]
    fn the_report_names_each_version_its_shares_and_the_verdict() {
        let mut all = vec![
            seen("a", 9, true, Some("ana@padaria.com.br")),
            seen("a", 20, false, Some("bia@gmail.com")),
        ];
        all.extend((0..48).map(|_| seen("a", 9, false, None)));
        all.extend((0..50).map(|_| seen("b", 9, false, None)));
        let report = render(&all);
        assert!(report.starts_with("test heading\n"), "{report}");
        let a = report
            .lines()
            .find(|line| line.trim_start().starts_with("a "))
            .unwrap();
        let words: Vec<&str> = a.split_whitespace().collect();
        assert_eq!(
            words,
            ["a", "50", "100%", "30", "2%", "4%", "0%", "50%"],
            "{a}"
        );
        assert!(report.contains("verdict: not ready: every version needs 50 more visitors"));
        assert!(report.contains("07:00-18:00  a 49 visitors"), "{report}");
        assert!(report.contains("18:00-07:00  a 1 visitors, prompt 0%, last section 100% (thin)"));
    }

    #[test]
    fn nothing_seen_says_so() {
        assert_eq!(render(&[]), "no landing test has had a visitor yet\n");
    }

    #[test]
    fn a_median_takes_the_lower_middle() {
        assert_eq!(median(vec![5, 1, 3]), "3");
        assert_eq!(median(vec![4, 1]), "1");
        assert_eq!(median(vec![]), "-");
    }
}
