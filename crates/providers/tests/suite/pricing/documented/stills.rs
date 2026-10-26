//! *Stills*: Nano Banana 2.1's per-picture and input rates, and the batch share.

use scorsese_core::ImageResolution;
use scorsese_providers::api::gemini::MODEL;
use scorsese_providers::prices::gemini;

use super::{checked_on, code, micros, prose, section, table};

const HEADER: &str = "| on the wire | 1K | 2K | 4K | input, per 1M tokens |";

#[test]
fn every_rate_per_still_is_the_codes() {
    let rows = table(section("Stills"), HEADER);
    let models: Vec<&str> = rows.iter().map(|row| code(&row[0])).collect();
    assert_eq!(models, [MODEL], "the Stills table's models");
    let row = &rows[0];
    assert_eq!(row.len(), ImageResolution::ALL.len() + 2, "a cell per size");
    for (cell, resolution) in row[1..].iter().zip(ImageResolution::ALL) {
        let rate = gemini::rate(resolution).expect("every size is priced");
        assert_eq!(
            micros(cell),
            rate.microdollars_per_image,
            "docs/prices.md, Stills, {}",
            resolution.as_str()
        );
    }
    let input = micros(row.last().expect("an input cell")) / 10_000;
    for size in gemini::RATES {
        assert_eq!(
            input,
            size.rate.cents_per_million_input,
            "docs/prices.md, Stills, input at {}",
            size.resolution.as_str()
        );
    }
}

#[test]
fn the_batch_share_is_the_codes() {
    let named = format!(
        "`prices::gemini::BATCH_PERCENT` is `{}`",
        gemini::BATCH_PERCENT
    );
    assert!(
        prose(section("Stills")).contains(&named),
        "docs/prices.md, Stills in a batch, should say {named}"
    );
}

#[test]
fn the_page_says_the_day_every_row_was_read() {
    let page = checked_on(section("Stills"));
    for row in gemini::RATES {
        assert_eq!(
            page,
            row.rate.checked.to_string(),
            "the stills' checked date"
        );
    }
}
