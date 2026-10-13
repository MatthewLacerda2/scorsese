//! What the environment has to say for Google sign-in to be on.

use scorsese_providers::credentials::Environment;
use scorsese_server::config::{CACHE, DATABASE_URL, STORAGE};
use scorsese_server::config::{GOOGLE_CLIENT_ID, GOOGLE_CLIENT_SECRET, PUBLIC_ORIGIN};
use scorsese_server::{Config, ConfigError};

fn with(extra: &[(&'static str, &'static str)]) -> Result<Config, ConfigError> {
    let mut pairs = vec![
        (DATABASE_URL, "postgres://db/scorsese"),
        (STORAGE, "/srv/scorsese"),
        (CACHE, "/srv/cache"),
    ];
    pairs.extend_from_slice(extra);
    Config::from_environment(&Environment::of(pairs))
}

const CLIENT: [(&str, &str); 2] = [
    (GOOGLE_CLIENT_ID, "id.apps.googleusercontent.com"),
    (GOOGLE_CLIENT_SECRET, "GOCSPX-hunter2"),
];

#[test]
fn all_three_settings_turn_it_on_and_the_secret_stays_hidden() {
    let config = with(&[
        CLIENT[0],
        CLIENT[1],
        (PUBLIC_ORIGIN, "https://scorseseestudio.com/"),
    ]);
    let google = config.unwrap().google.expect("configured");
    assert_eq!(
        google.redirect_uri(),
        "https://scorseseestudio.com/api/auth/google/callback"
    );
    assert_eq!(google.client.secret.expose(), "GOCSPX-hunter2");
    assert!(!format!("{google:?}").contains("hunter2"));
}

#[test]
fn any_one_missing_leaves_it_off_and_the_server_starting() {
    assert!(with(&[]).unwrap().google.is_none());
    for missing in 0..3 {
        let all = [
            CLIENT[0],
            CLIENT[1],
            (PUBLIC_ORIGIN, "https://scorseseestudio.com"),
        ];
        let some: Vec<_> = (0..3).filter(|i| *i != missing).map(|i| all[i]).collect();
        assert!(
            with(&some).unwrap().google.is_none(),
            "without {}",
            all[missing].0
        );
    }
}

#[test]
fn the_origin_is_https_or_this_machine_and_nothing_more() {
    for good in [
        "https://scorseseestudio.com",
        "http://localhost:5173",
        "http://127.0.0.1:8080",
    ] {
        let config = with(&[CLIENT[0], CLIENT[1], (PUBLIC_ORIGIN, good)]).unwrap();
        assert_eq!(config.google.unwrap().origin, good);
    }
    for bad in [
        "scorseseestudio.com",
        "http://scorseseestudio.com",
        "https://scorseseestudio.com/app",
        "https://",
        "http://localhost.evil.example",
    ] {
        let error = with(&[CLIENT[0], CLIENT[1], (PUBLIC_ORIGIN, bad)]).unwrap_err();
        assert!(
            matches!(error, ConfigError::Origin { .. }),
            "{bad}: {error}"
        );
    }
}
