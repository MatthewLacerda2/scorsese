//! *Narration*: ElevenLabs' rate per thousand characters, and a line's price.

use scorsese_core::SpeechModel;
use scorsese_providers::prices::{dollars, elevenlabs, speech};

use super::{checked_on, code, micros, section, table};

const HEADER: &str = "| model | on the wire | per 1000 characters | a 200-character line |";

#[test]
fn every_rate_per_character_is_the_codes() {
    let rows = table(section("Narration"), HEADER);
    let named: Vec<&str> = rows.iter().map(|row| code(&row[0])).collect();
    let coded: Vec<&str> = SpeechModel::ALL
        .map(|model| elevenlabs::Model::from(model).label())
        .to_vec();
    assert_eq!(named, coded, "the Narration table's models");
    for (row, spoken) in rows.iter().zip(SpeechModel::ALL) {
        let model = elevenlabs::Model::from(spoken);
        let rate = elevenlabs::rate(model).expect("every model is priced");
        let at = |column| format!("docs/prices.md, Narration, `{}`, {column}", model.label());
        assert_eq!(code(&row[1]), model.model_id(), "{}", at("on the wire"));
        assert_eq!(
            micros(&row[2]) / 10_000,
            rate.cents_per_1k_chars,
            "{}",
            at("per 1000 characters")
        );
        let line = speech(spoken, 200).expect("priced").cents;
        assert_eq!(row[3], dollars(line), "{}", at("a 200-character line"));
    }
}

#[test]
fn the_page_says_the_day_every_row_was_read() {
    let page = checked_on(section("Narration"));
    for row in elevenlabs::RATES {
        assert_eq!(
            page,
            row.rate.checked.to_string(),
            "narration's checked date"
        );
    }
}
