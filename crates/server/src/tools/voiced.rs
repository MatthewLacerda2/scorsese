//! A designed voice is its maker's alone (#687): the two places a voice id
//! reaches the vendor from the web, held to [`owners`].
//!
//! `generate` would narrate with it, and `voices` with `check` would look it up
//! and say what it is — the registry's `voices` runs unchanged, so its `check`
//! is answered here first. The listings themselves need nothing: they are the
//! built-in set and the Voice Library, and the account's designed voices are in
//! neither.

use serde_json::Value;

use super::{Caller, database};
use crate::designs::owners;

/// Refuse a `voices` call whose `check` names another user's designed voice.
pub(super) async fn checked(caller: &Caller<'_>, arguments: &Value) -> Result<(), String> {
    let Some(voice) = arguments
        .get("check")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|voice| !voice.is_empty())
    else {
        return Ok(());
    };
    unused_by_others(caller, &[voice.to_owned()]).await
}

/// Refuse when any of `voices` is another user's designed voice.
pub(super) async fn unused_by_others(caller: &Caller<'_>, voices: &[String]) -> Result<(), String> {
    let others = owners::others(&caller.toolbox.pool, caller.user, voices)
        .await
        .map_err(database)?;
    match others.first() {
        Some(voice) => Err(owners::refusal(voice)),
        None => Ok(()),
    }
}
