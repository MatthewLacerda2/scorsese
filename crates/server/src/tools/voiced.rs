//! A custom voice is its maker's alone (#687, #1655): the two places a voice
//! id reaches the vendor from the web, held to [`owners`].
//!
//! `generate` would narrate with it, and `voices` with `check` would look it up
//! and say what it is — the registry's `voices` runs unchanged, so its `check`
//! is answered here first. The listings themselves need nothing: they are the
//! built-in set and the Voice Library, and the account's custom voices are in
//! neither.
//!
//! An allow-list: a voice the caller keeps is theirs; one another user keeps
//! is refused without asking anybody; every other is asked of the vendor
//! ([`Catalogue::origin`]) and goes through only as a built-in or Voice
//! Library voice. A vendor that cannot be asked refuses the call: who may use
//! a voice is never guessed. An id nothing answers to goes through, to be
//! told so by `voices` or to fail its job unpaid, as it always has.

use std::collections::BTreeSet;

use scorsese_providers::voices::Origin;
use serde_json::Value;

use super::{Caller, database};
use crate::designs::owners;

/// Refuse a `voices` call whose `check` names a voice the caller may not use.
pub(super) async fn checked(caller: &Caller<'_>, arguments: &Value) -> Result<(), String> {
    let Some(voice) = arguments
        .get("check")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|voice| !voice.is_empty())
    else {
        return Ok(());
    };
    usable(caller, &[voice.to_owned()]).await
}

/// Refuse when any of `voices` is a custom voice the caller does not keep.
pub(super) async fn usable(caller: &Caller<'_>, voices: &[String]) -> Result<(), String> {
    let named = owners::named(&caller.toolbox.pool, caller.user, voices)
        .await
        .map_err(database)?;
    if let Some(voice) = named.others.first() {
        return Err(owners::refusal(voice));
    }
    let asked: BTreeSet<String> = voices
        .iter()
        .filter(|voice| !named.mine.contains(voice))
        .cloned()
        .collect();
    if asked.is_empty() {
        return Ok(());
    }
    let catalogue = caller.toolbox.catalogue()?;
    let answers = tokio::task::spawn_blocking(move || {
        asked
            .into_iter()
            .map(|voice| {
                let origin = catalogue.origin(&voice);
                (voice, origin)
            })
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|error| format!("the voice check stopped: {error}"))?;
    for (voice, origin) in answers {
        match origin {
            Ok(Some(Origin::Builtin | Origin::Library) | None) => {}
            Ok(Some(Origin::Custom)) => return Err(owners::not_kept(&voice)),
            Err(error) => {
                return Err(format!(
                    "voice {voice} could not be checked with ElevenLabs, so nothing was \
                     quoted or charged: {error}. Try again in a moment."
                ));
            }
        }
    }
    Ok(())
}
