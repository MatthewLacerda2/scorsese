//! *Music*: Eleven Music's rate per minute, and a track's price (#1264).

use scorsese_providers::api::elevenlabs::music::MODEL_ID;
use scorsese_providers::prices::{dollars, elevenlabs, music};

use super::{checked_on, code, micros, section, table};

const HEADER: &str = "| model | on the wire | per minute | a 40-second track |";

#[test]
fn the_rate_per_minute_is_the_codes() {
    let rows = table(section("Music"), HEADER);
    assert_eq!(rows.len(), 1, "the Music table is one row");
    let row = &rows[0];
    assert_eq!(code(&row[1]), MODEL_ID, "Music, on the wire");
    assert_eq!(
        micros(&row[2]) / 10_000,
        elevenlabs::MUSIC.cents_per_minute,
        "Music, per minute"
    );
    assert_eq!(row[3], dollars(music(40).cents), "Music, a 40-second track");
}

#[test]
fn the_page_says_the_day_the_rate_was_read() {
    assert_eq!(
        checked_on(section("Music")),
        elevenlabs::MUSIC.checked.to_string(),
        "music's checked date"
    );
}

#[test]
fn a_track_rounds_up_to_the_cent() {
    assert_eq!(music(40).cents, 10);
    assert_eq!(music(41).cents, 11);
    assert_eq!(music(600).cents, 150);
}
