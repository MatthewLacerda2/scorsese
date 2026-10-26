//! Where the Whisper server is: the environment, the settings file, or the
//! default.

use scorsese_providers::credentials::{Environment, Settings};
use scorsese_providers::transcribe::{DEFAULT_ADDRESS, VARIABLE, address_from};

#[test]
fn the_variable_wins_then_the_settings_file_then_the_default() {
    let saved = Settings {
        whisper_url: Some(" http://box:9000 ".into()),
        ..Settings::default()
    };
    let exported = Environment::of([(VARIABLE, "http://whisper:8000")]);
    assert_eq!(address_from(&exported, &saved), "http://whisper:8000");
    assert_eq!(
        address_from(&Environment::default(), &saved),
        "http://box:9000"
    );
    assert_eq!(
        address_from(&Environment::of([(VARIABLE, "  ")]), &Settings::default()),
        DEFAULT_ADDRESS
    );
    assert_eq!(DEFAULT_ADDRESS, "http://127.0.0.1:8970");
    assert_eq!(VARIABLE, "SCORSESE_WHISPER_URL");
}
