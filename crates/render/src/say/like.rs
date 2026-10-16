//! A measurement held against a reference record (#1125).
//!
//! `docs/references.md` used to ask the reader to put a bake's section rows
//! beside a table by hand. These lines do it: the bake's **body** — its
//! sections after the first and before the last, as the record's body is its
//! windows after the intro and before the fade — field by field against the
//! record's ranges, and its opening against how the record opens.
//!
//! Each field says its spread across the body, the record's spread, and a
//! verdict in words: how many body sections fall outside, and which way that
//! reads. A word rather than a sign, for the reason [`super::comparison`]
//! gives: under on crest is *flatter*, under on correlation is *wider*.
//!
//! The mean gets no verdict. The records' masters run over full scale and a
//! bake is held at −1 dBTP, so the mean a bake can reach is the one its crest
//! leaves it, and the row says that number instead.

use scorsese_zimmer::level::{Profile, Span};

use super::moment;
use crate::audio::reference::{Range, Reference};

/// How the body of `profile` sits against `reference`, as lines to print
/// under the report it belongs to.
pub fn likeness(profile: &Profile, reference: &Reference) -> Vec<String> {
    let body = body(profile);
    let mut lines = vec![head(reference, &body)];

    let crests: Vec<f64> = body.iter().filter_map(|span| span.crest_db()).collect();
    let aim = reference
        .aim_crest
        .map_or_else(String::new, |crest| format!(", aim ~{crest:.1}"));
    lines.push(field(Field {
        name: "crest",
        values: &crests,
        record: reference.crest,
        judged: reference.crest,
        decimals: 1,
        signed: false,
        extra: &aim,
        words: ("flatter than the record", "more dynamic than the record"),
    }));

    let shares: Vec<(u32, u32, u32)> = body
        .iter()
        .filter_map(|span| span.bands.map(|bands| bands.percentages()))
        .collect();
    let share = |pick: fn(&(u32, u32, u32)) -> u32| -> Vec<f64> {
        shares.iter().map(|it| f64::from(pick(it))).collect()
    };
    lines.push(field(Field {
        name: "low %",
        values: &share(|it| it.0),
        record: reference.low,
        judged: reference.low,
        decimals: 0,
        signed: false,
        extra: "",
        words: ("lighter than the record", "heavier than the record"),
    }));
    lines.push(field(Field {
        name: "mid %",
        values: &share(|it| it.1),
        record: reference.mid,
        judged: reference.mid,
        decimals: 0,
        signed: false,
        extra: "",
        words: ("less mid than the record", "boxier than the record"),
    }));
    // Only too bright is a finding: the records' files read low up there
    // (`docs/references.md`, "The files themselves"), so a bake under their
    // high share cannot be compared with them.
    lines.push(field(Field {
        name: "high %",
        values: &share(|it| it.2),
        record: reference.high,
        judged: Range {
            low: f64::NEG_INFINITY,
            high: reference.high.high,
        },
        decimals: 0,
        signed: false,
        extra: "",
        words: ("", "brighter than the record"),
    }));

    lines.push(match reference.correlation {
        Some(record) => {
            let widths: Vec<f64> = body.iter().filter_map(|span| span.correlation).collect();
            field(Field {
                name: "corr",
                values: &widths,
                record,
                judged: record,
                decimals: 2,
                signed: true,
                extra: "",
                words: ("wider than the record", "narrower than the record"),
            })
        }
        None => "corr     the record's file is mono, so its width says nothing".to_owned(),
    });

    lines.push(mean(&body, reference));
    if let Some(opening) = opening(profile, &body, reference) {
        lines.push(opening);
    }
    lines
}

/// The stretch a record's body corresponds to: every section but the first
/// and the last when there are three or more, all of them when there are two,
/// and the whole file when it was never split.
fn body(profile: &Profile) -> Vec<&Span> {
    match profile.sections.as_slice() {
        [] => vec![&profile.whole],
        sections @ ([_] | [_, _]) => sections.iter().collect(),
        [_, middle @ .., _] => middle.iter().collect(),
    }
}

/// The line naming the record and which stretch of this file is its body.
fn head(reference: &Reference, body: &[&Span]) -> String {
    let (Some(first), Some(last)) = (body.first(), body.last()) else {
        return format!("like {}", reference.row);
    };
    let count = body.len();
    let sections = if count == 1 { "section" } else { "sections" };
    format!(
        "like {} — body {}-{} ({count} {sections}); a floor to clear, never a target",
        reference.row,
        moment(first.from_seconds),
        moment(last.to_seconds),
    )
}

/// One field's row, and what it needs to say it.
struct Field<'a> {
    name: &'a str,
    /// The field in each body section that has it.
    values: &'a [f64],
    /// The record's spread, as printed.
    record: Range,
    /// The spread a value has to leave to be called out — the record's own,
    /// except where only one side of it can be compared.
    judged: Range,
    decimals: usize,
    /// Whether a value carries its sign, as a correlation does.
    signed: bool,
    /// Said after the record's spread.
    extra: &'a str,
    /// What being under, and over, [`Self::judged`] reads as.
    words: (&'a str, &'a str),
}

fn field(field: Field<'_>) -> String {
    let places = field.decimals;
    let spread = |low: f64, high: f64| {
        if field.signed {
            format!("{low:+.places$} to {high:+.places$}")
        } else {
            format!("{low:.places$} to {high:.places$}")
        }
    };
    let Some((low, high)) = extent(field.values) else {
        return format!("{:<8} nothing measured in the body", field.name);
    };
    let record = format!(
        "record {}{}",
        spread(field.record.low, field.record.high),
        field.extra
    );
    let count = field.values.len();
    let under = field
        .values
        .iter()
        .filter(|it| **it < field.judged.low)
        .count();
    let over = field
        .values
        .iter()
        .filter(|it| **it > field.judged.high)
        .count();
    let mut verdicts = Vec::new();
    for (outside, words) in [(under, field.words.0), (over, field.words.1)] {
        if outside > 0 {
            verdicts.push(format!("{outside} of {count} {words}"));
        }
    }
    let verdict = if verdicts.is_empty() {
        "inside".to_owned()
    } else {
        verdicts.join(", ")
    };
    format!(
        "{:<8} {:<18} {record:<32} {verdict}",
        field.name,
        spread(low, high)
    )
}

/// The mean row: where the body sits, and the mean a bake can reach.
fn mean(body: &[&Span], reference: &Reference) -> String {
    let means: Vec<f64> = body
        .iter()
        .filter_map(|span| span.loudness.mean_dbfs)
        .collect();
    let Some((low, high)) = extent(&means) else {
        return "mean     silent".to_owned();
    };
    let reachable = reference.reachable_mean().map_or_else(String::new, |mean| {
        format!("; at its aimed crest a bake lands near {mean:.1}")
    });
    format!(
        "mean     {:<18} record {:.1} to {:.1} dBFS, over full scale{reachable}",
        format!("{low:.1} to {high:.1}"),
        reference.mean.low,
        reference.mean.high
    )
}

/// How far an opening may sit further under the body than the record's does
/// before it is called quieter, in dB. A window's level wobbles by about this
/// much on its own, and Billie Jean's ~2 dB plus it is the "more than 3 dB
/// under" the anti-reference failed by.
const OPENING_SLACK_DB: f64 = 1.0;

/// How much louder than the record's an opening has to be before it is called
/// out — far wider than [`OPENING_SLACK_DB`], because opening strong is rarely
/// the mistake: an intro a few dB under its body (Brutal Doom's 3.5) is not a
/// shape to copy, and a build that is not there (Fuego's, 10 dB and more) is.
const OPENING_LOUDER_DB: f64 = 6.0;

/// The first section's mean against the body's, beside how the record opens.
/// `None` when the file was never split, since then it has no opening apart
/// from its body.
fn opening(profile: &Profile, body: &[&Span], reference: &Reference) -> Option<String> {
    if profile.sections.len() < 2 {
        return None;
    }
    let first = profile.sections.first()?.loudness.mean_dbfs?;
    let under = energy_mean(body)? - first;
    let record = reference.opening_under_db;
    let verdict = if under > record + OPENING_SLACK_DB {
        "quieter than the record opens"
    } else if under < record - OPENING_LOUDER_DB {
        "louder than the record opens"
    } else {
        "inside"
    };
    Some(format!(
        "opening  {first:.1} dBFS, {} the body  record ~{} the body  {verdict}",
        gap(under),
        gap(record)
    ))
}

/// A distance from the body, said the way round it goes.
fn gap(under: f64) -> String {
    if under < 0.0 {
        format!("{:.1} dB over", -under)
    } else {
        format!("{under:.1} dB under")
    }
}

/// The body's mean as one level: the sections' energy, weighted by how long
/// each runs, rather than an average of decibels.
fn energy_mean(body: &[&Span]) -> Option<f64> {
    let (energy, seconds) = body
        .iter()
        .filter_map(|span| Some((span.loudness.mean_dbfs?, span.seconds())))
        .fold((0.0, 0.0), |(energy, total), (mean, seconds)| {
            (energy + 10f64.powf(mean / 10.0) * seconds, total + seconds)
        });
    (seconds > 0.0).then(|| 10.0 * (energy / seconds).log10())
}

/// The lowest and highest of `values`, or `None` when there are none.
fn extent(values: &[f64]) -> Option<(f64, f64)> {
    let low = values.iter().copied().reduce(f64::min)?;
    let high = values.iter().copied().reduce(f64::max)?;
    Some((low, high))
}
