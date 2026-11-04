//! *Sound effects*: ElevenLabs Sound Effects' rate per minute, and a sound's
//! price (#1265).

use scorsese_providers::api::elevenlabs::sound::MODEL_ID;
use scorsese_providers::prices::{dollars, elevenlabs, from_cents, sound};

use super::{checked_on, code, micros, section, table};

const HEADER: &str = "| model | on the wire | per minute | a 5-second sound |";

#[test]
fn the_rate_per_minute_is_the_codes() {
    let rows = table(section("Sound effects"), HEADER);
    assert_eq!(rows.len(), 1, "the Sound effects table is one row");
    let row = &rows[0];
    assert_eq!(code(&row[1]), MODEL_ID, "Sound effects, on the wire");
    assert_eq!(
        micros(&row[2]) / 10_000,
        elevenlabs::SOUND.cents_per_minute,
        "Sound effects, per minute"
    );
    assert_eq!(
        row[3],
        dollars(from_cents(sound(5_000).cents), 2),
        "Sound effects, a 5-second sound"
    );
}

#[test]
fn the_page_says_the_day_the_rate_was_read() {
    assert_eq!(
        checked_on(section("Sound effects")),
        elevenlabs::SOUND.checked.to_string(),
        "sound effects' checked date"
    );
}

#[test]
fn a_sound_rounds_up_to_the_cent() {
    assert_eq!(sound(500).cents, 1);
    assert_eq!(sound(5_000).cents, 1);
    assert_eq!(sound(5_001).cents, 2);
    assert_eq!(sound(30_000).cents, 6);
}
