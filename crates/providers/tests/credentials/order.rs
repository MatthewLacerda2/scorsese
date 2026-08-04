//! The resolution order, with both halves supplied rather than ambient.

use scorsese_providers::credentials::{
    Environment, Provider, Secret, Settings, Source, resolve_from,
};

/// A settings file carrying a Gemini key and nothing else.
fn saved(key: &str) -> Settings {
    Settings {
        gemini_api_key: Some(key.to_owned()),
        ..Settings::default()
    }
}

#[test]
fn an_exported_variable_wins_over_the_settings_file() {
    let environment = Environment::of([("GEMINI_API_KEY", "from-the-shell")]);
    let found = resolve_from(Provider::Gemini, &environment, &saved("from-the-file"))
        .expect("both places have one");
    assert_eq!(found.secret, Secret::new("from-the-shell"));
    assert_eq!(found.source, Source::Environment);
}

#[test]
fn the_settings_file_answers_when_the_environment_does_not() {
    let found = resolve_from(
        Provider::Gemini,
        &Environment::default(),
        &saved("from-the-file"),
    )
    .expect("the file has one");
    assert_eq!(found.secret, Secret::new("from-the-file"));
    assert!(matches!(found.source, Source::Settings(_)));
}

/// A variable set to nothing is a variable somebody meant to fill in, not a
/// credential — so it falls through rather than resolving to an empty key and
/// failing later against the provider.
#[test]
fn a_blank_variable_falls_through_rather_than_resolving_to_nothing() {
    let environment = Environment::of([("GEMINI_API_KEY", "   ")]);
    let found = resolve_from(Provider::Gemini, &environment, &saved("from-the-file"))
        .expect("the file still has one");
    assert_eq!(found.secret, Secret::new("from-the-file"));
}

#[test]
fn a_blank_saved_key_is_no_key_at_all() {
    resolve_from(Provider::Gemini, &Environment::default(), &saved("  "))
        .expect_err("a field somebody left empty is not a credential");
}

#[test]
fn each_provider_reads_its_own_variable() {
    let environment = Environment::of([("GEMINI_API_KEY", "video-key")]);
    let settings = Settings::default();
    resolve_from(Provider::Gemini, &environment, &settings).expect("gemini resolves");
    resolve_from(Provider::ElevenLabs, &environment, &settings)
        .expect_err("one key is not both keys");
}

/// A missing key is almost always a key that is somewhere else, so the refusal
/// lists everywhere that was tried rather than only reporting the absence.
#[test]
fn a_missing_key_says_everywhere_it_looked() {
    let refused = resolve_from(
        Provider::Gemini,
        &Environment::default(),
        &Settings::default(),
    )
    .expect_err("nowhere has one");
    let message = refused.to_string();
    assert!(message.contains("GEMINI_API_KEY"), "{message}");
    assert!(message.contains("Gemini"), "{message}");
    assert!(message.contains("settings.json"), "{message}");
}

/// The wrapper exists so that a key cannot reach a log through the back door
/// that `Debug` opens on every other type.
#[test]
fn printing_a_key_prints_nothing() {
    let secret = Secret::new("sk-do-not-log-me");
    assert!(!format!("{secret:?}").contains("do-not-log-me"));
}
