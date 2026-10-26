//! *Shots*: Veo's per-second table, and the per-length one derived from it.

use scorsese_core::{ClipSeconds, VideoModel, VideoRequest, VideoResolution};
use scorsese_providers::prices::{Quality, Tier, dollars, estimate, from_cents, veo};

use super::{checked_on, micros, section, table};

const PER_SECOND: &str = "| | 720p | 1080p | 4k |";
const PER_LENGTH: &str = "| | 4s | 6s | 8s |";

/// What the per-second table says a cell is: a rate, or nothing sold.
fn per_second(cell: &str) -> Option<u64> {
    (cell != "*not sold*").then(|| micros(cell) / 10_000)
}

#[test]
fn every_rate_per_second_is_the_codes() {
    let rows = table(section("Shots"), PER_SECOND);
    let tiers: Vec<&str> = rows.iter().map(|row| row[0].as_str()).collect();
    let coded: Vec<&str> = VideoModel::ALL
        .map(|model| Tier::from(model).label())
        .to_vec();
    assert_eq!(tiers, coded, "the Shots table's tiers are not the code's");
    for (row, model) in rows.iter().zip(VideoModel::ALL) {
        let tier = Tier::from(model);
        for (cell, resolution) in row[1..].iter().zip(VideoResolution::ALL) {
            let quality = Quality::from(resolution);
            assert_eq!(
                per_second(cell),
                veo::rate(tier, quality).map(|rate| rate.cents_per_second),
                "docs/prices.md, Shots, `{}` at {}: the page says {cell}",
                tier.label(),
                quality.label(),
            );
        }
    }
}

/// A row per tier and size Google sells, a cell per length Veo makes there.
#[test]
fn every_shot_by_length_is_what_the_code_estimates() {
    let rows = table(section("Shots"), PER_LENGTH);
    let mut expected = Vec::new();
    for model in VideoModel::ALL {
        for resolution in VideoResolution::ALL {
            if model.supports(resolution) {
                expected.push(name(model, resolution));
            }
        }
    }
    let named: Vec<String> = rows.iter().map(|row| row[0].clone()).collect();
    assert_eq!(named, expected, "the per-length table's rows");
    for row in &rows {
        let (model, resolution) = parse(&row[0]);
        for (cell, seconds) in row[1..].iter().zip(ClipSeconds::ALL) {
            let made = !resolution.locks_length() || seconds == ClipSeconds::Eight;
            let request = VideoRequest {
                model,
                resolution,
                seconds,
                ..VideoRequest::default()
            };
            let code = match made {
                true => dollars(from_cents(estimate(&request).expect("sold").cents), 2),
                false => "—".to_owned(),
            };
            assert_eq!(
                *cell,
                code,
                "docs/prices.md, Shots, `{}` at {}s",
                row[0],
                seconds.get()
            );
        }
    }
}

#[test]
fn the_page_says_the_day_every_row_was_read() {
    let page = checked_on(section("Shots"));
    for row in veo::RATES {
        assert_eq!(page, row.rate.checked.to_string(), "Veo's checked date");
    }
}

/// `Standard, 720p`: the tier as a word and the size as the format spells it.
fn name(model: VideoModel, resolution: VideoResolution) -> String {
    let tier = model.as_str();
    let capital = tier[..1].to_uppercase() + &tier[1..];
    format!("{capital}, {}", resolution.as_str())
}

fn parse(row: &str) -> (VideoModel, VideoResolution) {
    VideoModel::ALL
        .into_iter()
        .flat_map(|model| VideoResolution::ALL.map(|resolution| (model, resolution)))
        .find(|(model, resolution)| name(*model, *resolution) == row)
        .unwrap_or_else(|| panic!("`{row}` names no tier and size"))
}
