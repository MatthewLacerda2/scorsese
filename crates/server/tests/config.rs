//! What the environment has to say for the server to start — and that the
//! database password never comes back out of it.

use scorsese_providers::credentials::Environment;
use scorsese_server::config::{BIND, DATABASE_URL, DEFAULT_BIND, STORAGE};
use scorsese_server::{Config, ConfigError};

const URL: &str = "postgres://scorsese:hunter2@db/scorsese";

fn with(extra: &[(&'static str, &'static str)]) -> Result<Config, ConfigError> {
    let mut pairs = vec![(DATABASE_URL, URL), (STORAGE, "/srv/scorsese")];
    for (name, value) in extra {
        pairs.retain(|(existing, _)| existing != name);
        pairs.push((name, value));
    }
    Config::from_environment(&Environment::of(pairs))
}

#[test]
fn a_complete_environment_listens_on_loopback_by_default() {
    let config = with(&[]).unwrap();
    assert_eq!(config.database_url.expose(), URL);
    assert_eq!(config.storage.to_str(), Some("/srv/scorsese"));
    assert_eq!(config.bind.to_string(), DEFAULT_BIND);
    assert!(config.bind.ip().is_loopback());
}

#[test]
fn the_bind_address_is_taken_when_given() {
    let config = with(&[(BIND, "0.0.0.0:9000")]).unwrap();
    assert_eq!(config.bind.to_string(), "0.0.0.0:9000");
}

#[test]
fn a_missing_or_blank_variable_is_named() {
    for variable in [DATABASE_URL, STORAGE] {
        for value in ["", "   "] {
            let error = with(&[(variable, value)]).unwrap_err();
            assert!(
                matches!(error, ConfigError::Missing { variable: named, .. } if named == variable),
                "{variable}={value:?} gave {error:?}"
            );
            assert!(error.to_string().starts_with(variable), "{error}");
        }
    }
}

#[test]
fn a_relative_storage_root_is_refused() {
    let error = with(&[(STORAGE, "data/files")]).unwrap_err();
    assert!(
        matches!(error, ConfigError::RelativeStorage { .. }),
        "{error:?}"
    );
    assert!(error.to_string().contains("data/files"), "{error}");
}

#[test]
fn an_unparseable_bind_address_is_refused() {
    let error = with(&[(BIND, "localhost")]).unwrap_err();
    assert!(matches!(error, ConfigError::Bind { .. }), "{error:?}");
}

#[test]
fn the_database_password_is_never_printed() {
    let config = with(&[]).unwrap();
    let printed = format!("{config:?}");
    assert!(!printed.contains("hunter2"), "{printed}");
    assert!(printed.contains("/srv/scorsese"), "{printed}");
}
