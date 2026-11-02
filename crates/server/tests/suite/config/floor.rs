//! How much disk must stay free (`SCORSESE_DISK_FLOOR_GB`, #1647): 10 GB
//! unless set, `0` for none, and anything else refused by name.

use scorsese_server::ConfigError;
use scorsese_server::config::DISK_FLOOR;

use super::with;

#[test]
fn the_disk_floor_is_ten_gigabytes_unless_set() {
    assert_eq!(with(&[]).unwrap().disk_floor.get(), 10_000_000_000);
    assert_eq!(
        with(&[(DISK_FLOOR, "25")]).unwrap().disk_floor.get(),
        25_000_000_000
    );
    assert_eq!(with(&[(DISK_FLOOR, "0")]).unwrap().disk_floor.get(), 0);
}

#[test]
fn a_floor_that_is_not_whole_gigabytes_is_refused_by_name() {
    for nonsense in ["10GB", "-1", "2.5", "lots"] {
        let error = with(&[(DISK_FLOOR, nonsense)]).unwrap_err();
        assert!(
            matches!(error, ConfigError::Floor { .. }),
            "{nonsense}: {error}"
        );
        assert!(error.to_string().starts_with(DISK_FLOOR), "{error}");
    }
}
