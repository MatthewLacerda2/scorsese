//! How much disk must stay free (`SCORSESE_DISK_FLOOR_GB`, #1647): anything
//! but whole gigabytes refused by name. The default and the parsing are
//! `config`'s own tests; this one names only what the base had, so CI's
//! regression job can build it there.

use super::with;

const DISK_FLOOR: &str = "SCORSESE_DISK_FLOOR_GB";

#[test]
fn a_floor_that_is_not_whole_gigabytes_is_refused_by_name() {
    for nonsense in ["10GB", "-1", "2.5", "lots"] {
        let refused = with(&[(DISK_FLOOR, nonsense)]);
        let error = refused.expect_err(nonsense);
        assert!(error.to_string().starts_with(DISK_FLOOR), "{error}");
    }
}
