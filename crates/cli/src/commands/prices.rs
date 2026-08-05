//! `scorsese prices` — what a generation costs, and when we last checked.
//!
//! Two vendors, two tables, one page. They are billed by different things — a
//! shot by its length, a line by its characters — so a single table would have
//! to carry a unit column and would read as though the numbers were
//! comparable. They are not: eight seconds of video is ninety-six cents and a
//! sentence of narration is one.

use anyhow::Result;
use scorsese_providers::prices::{Checked, STALE_AFTER_DAYS, dollars, elevenlabs, veo};

/// Prints the rate table, as Markdown.
///
/// Markdown because this has two readers and one of them is a CI job summary.
/// A pipe-delimited table reads perfectly well in a terminal — the vendor's own
/// price list looks like this — so the alternative was a second rendering that
/// could disagree with the first.
///
/// It prints every published rate, including the tiers scorsese does not offer,
/// with those marked. The table exists to be checked off against Google's page,
/// and rows missing from it are rows nobody ticks.
pub(crate) fn run() -> Result<()> {
    let today = Checked::today();

    println!("Veo 3.1, per second of finished video. Paid tier; there is no free tier.");
    println!();
    println!("| tier | size | per second | 8 seconds | checked |");
    println!("| --- | --- | --- | --- | --- |");
    for row in veo::RATES {
        let offered = row.tier.is_offered() && row.quality.is_offered();
        let label = if offered {
            row.tier.label().to_owned()
        } else {
            format!("{} *(not offered)*", row.tier.label())
        };
        println!(
            "| {label} | {} | {} | {} | {}{} |",
            row.quality.label(),
            dollars(row.rate.cents_per_second),
            dollars(row.rate.cents_per_second * 8),
            row.rate.checked,
            if row.rate.checked.is_stale_on(today) {
                " ⚠️"
            } else {
                ""
            },
        );
    }

    println!();
    println!("ElevenLabs text-to-speech, per thousand characters of input text.");
    println!();
    println!("| model | on the wire | per 1000 characters | a 200-character line | checked |");
    println!("| --- | --- | --- | --- | --- |");
    for row in elevenlabs::RATES {
        println!(
            "| {} | `{}` | {} | {} | {}{} |",
            row.model.label(),
            row.model.model_id(),
            dollars(row.rate.cents_per_1k_chars),
            dollars(row.rate.cents_per_1k_chars.div_ceil(5)),
            row.rate.checked,
            if row.rate.checked.is_stale_on(today) {
                " ⚠️"
            } else {
                ""
            },
        );
    }

    println!();
    stale_note(today);
    println!();
    println!("Nobody bills these back. No provider reports what a generation cost, so every");
    println!("figure scorsese records is its own arithmetic over this table — see docs/prices.md.");
    Ok(())
}

/// The line about how old the figures are, or that they are not.
///
/// Informational, always — a stale price is a price worth re-reading, not a
/// reason to stop anybody generating anything. It never sets an exit code.
fn stale_note(today: Checked) {
    // Across both tables, because the reader wants one answer to "how old is
    // any of this" rather than a date per vendor to compare themselves.
    let oldest = veo::RATES
        .iter()
        .map(|row| row.rate.checked)
        .chain(elevenlabs::RATES.iter().map(|row| row.rate.checked))
        .min();
    let Some(oldest) = oldest else {
        return;
    };
    let days = oldest.days_until(today);
    let ago = if days == 1 {
        String::from("1 day")
    } else {
        format!("{days} days")
    };
    if oldest.is_stale_on(today) {
        println!(
            "⚠️ The oldest figure here was checked {ago} ago, over the {STALE_AFTER_DAYS}-day \
             mark. Worth re-reading <https://ai.google.dev/gemini-api/docs/pricing> \
             and <https://elevenlabs.io/pricing>."
        );
    } else {
        println!("Oldest figure checked {ago} ago; the mark is {STALE_AFTER_DAYS} days.");
    }
}
