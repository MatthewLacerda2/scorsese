//! How much of the shared machine one account may take (`config::PerUser`):
//! a default for each, and a setting that is not one refused by name.

use scorsese_server::ConfigError;
use scorsese_server::config::{USER_BATCH_STILLS, USER_STORAGE, USER_TURNS};

use super::with;

#[test]
fn each_account_s_share_of_the_machine_has_a_default_and_can_be_set() {
    let config = with(&[]).unwrap();
    assert_eq!(config.per_user.turns, 2);
    assert_eq!(config.per_user.batch_stills, 16);
    assert_eq!(config.per_user.storage.get(), 50_000_000_000);
    assert_eq!(config.files().quota().get(), 50_000_000_000);

    let set = with(&[
        (USER_TURNS, "1"),
        (USER_BATCH_STILLS, "4"),
        (USER_STORAGE, "200GB"),
    ])
    .unwrap();
    assert_eq!(set.per_user.turns, 1);
    assert_eq!(set.per_user.batch_stills, 4);
    assert_eq!(set.files().quota().get(), 200_000_000_000);
}

#[test]
fn a_share_that_is_not_one_is_refused_by_name() {
    for variable in [USER_TURNS, USER_BATCH_STILLS] {
        for nonsense in ["0", "-1", "two"] {
            let error = with(&[(variable, nonsense)]).unwrap_err();
            assert!(
                matches!(error, ConfigError::Count { variable: named, .. } if named == variable),
                "{variable}={nonsense}: {error}"
            );
        }
    }
    for nonsense in ["0", "lots", "20 GiB"] {
        let error = with(&[(USER_STORAGE, nonsense)]).unwrap_err();
        assert!(
            error.to_string().starts_with(USER_STORAGE),
            "{nonsense}: {error}"
        );
    }
}
