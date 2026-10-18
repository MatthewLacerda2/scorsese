//! `voice_clone` on the hosted server (#1263): the same consent, the same
//! rules on what may be cloned from, and the same free price as the stdio
//! tool — with the recordings the user's own library files, imported into
//! their project like any other, and the clone made by the job queue under
//! the voice-slot rule (`crate::designs::clones`).

use scorsese_core::{AssetId, AssetKind};
use scorsese_mcp::Reply;
use scorsese_providers::voices::clone::{CONSENT, check};
use serde_json::{Value, json};

use super::super::{Caller, database, project_id};
use super::args::text;
use crate::db;
use crate::designs::clones::{self, ClonePayload, Recorded};
use crate::designs::forget::ForgetPayload;
use crate::jobs::{kinds, store as jobs};
use crate::projects;

/// How a client names it.
pub(crate) const NAME: &str = "voice_clone";

/// What it does.
pub(crate) const DESCRIPTION: &str = "Clone a voice from recordings already in one of your \
projects, so a video can be narrated in your own voice: ElevenLabs' Instant Voice Cloning. Clone \
only your own voice, or the voice of someone who has given you permission to clone it — ask \
whoever is speaking in the recordings in those words before calling, and pass consent only once \
they have said yes. A cloned voice is bound by ElevenLabs' terms of use \
(https://elevenlabs.io/terms-of-use) and its prohibited use policy \
(https://elevenlabs.io/use-policy): never one to impersonate, mislead or harm anyone. Upload the \
recordings to your library and import them into the project first, then pass the project, the \
audio assets (about one to two minutes of clear speech in all), a name and consent. Free — \
creating a clone costs nothing, and what it narrates is paid for as any line is — but it takes one \
of the few voices you may keep at once, the same as a designed one. It runs as a job whose result \
is the voice_id a narration names. Pass list to read back the voices you have cloned, and forget \
with one of their voice_ids to delete it and free its slot — free, a job.";

/// Its arguments.
pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "project": {
                "type": "integer",
                "description": "The project whose recordings to clone from, by id, as \
                                project_list shows it. Needed to clone; not for list or forget."
            },
            "assets": {
                "type": "array",
                "items": { "type": "string" },
                "description": "The ids of the audio assets to clone from, as project_read \
                                shows them: recordings you uploaded of the one person whose \
                                voice it is, speaking clearly, with no music, reverb or \
                                background noise. About one to two minutes in all is what \
                                ElevenLabs recommends; more than three adds little."
            },
            "name": {
                "type": "string",
                "description": "What to call the voice — what it is recognisable by in every \
                                voice listing, so say whose voice it is."
            },
            "description": {
                "type": "string",
                "description": "What the voice is, in a sentence, kept with it. Optional."
            },
            "remove_background_noise": {
                "type": "boolean",
                "description": "Ask ElevenLabs to clean background noise out of the \
                                recordings first. Only for recordings that have some: on clean \
                                ones it can lower the quality."
            },
            "consent": {
                "type": "boolean",
                "description": "True only once the person speaking in the recordings has said \
                                the voice is their own, or that its owner has given permission \
                                to clone it. Ask in those words; never assume it. Without it \
                                nothing is read or sent."
            },
            "list": {
                "type": "boolean",
                "description": "Read back every voice you have cloned, with the recordings \
                                each was made from. Spends nothing."
            },
            "forget": {
                "type": "string",
                "description": "The voice_id of a voice you cloned, exactly as list shows it. \
                                Deletes it from scorsese's ElevenLabs account and frees its \
                                slot. Free, a job. Cannot be undone at the vendor, but the \
                                recordings stay in your library, so cloning them again makes \
                                a voice like it."
            }
        }
    })
}

/// Clone, list or forget.
pub(crate) async fn call(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    if arguments.get("list").and_then(Value::as_bool) == Some(true) {
        return list(caller).await;
    }
    if let Some(voice) = text(arguments, "forget") {
        return forget(caller, &voice).await;
    }
    let consent = arguments.get("consent").and_then(Value::as_bool) == Some(true);
    let name = text(arguments, "name").unwrap_or_default();
    check(consent, &name).map_err(|error| error.to_string())?;
    let recordings = recordings(caller, arguments).await?;
    let payload = ClonePayload {
        name: name.clone(),
        description: text(arguments, "description"),
        remove_background_noise: arguments
            .get("remove_background_noise")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        recordings,
    };
    let payload = serde_json::to_value(&payload).map_err(database)?;
    let mut tx = db::scoped(&caller.toolbox.pool, caller.user)
        .await
        .map_err(database)?;
    let job = jobs::enqueue_by(&mut tx, kinds::VOICE_KEEP, &payload, Some(caller.call))
        .await
        .map_err(database)?;
    tx.commit().await.map_err(database)?;
    caller.toolbox.queue.announce(caller.user, &job);
    Ok(format!(
        "Cloning \"{name}\" as job {} — free. Call jobs with job: {}; its result is the voice_id \
         a narration names. The voice lives in scorsese's ElevenLabs account, not in any \
         project, so it is yours in every project. {CONSENT}",
        job.id, job.id
    )
    .into())
}

/// The library files the named assets of the caller's project are — refused
/// unless each is an uploaded audio file of theirs.
async fn recordings(caller: &Caller<'_>, arguments: &Value) -> Result<Vec<Recorded>, String> {
    let ids: Vec<AssetId> = arguments
        .get("assets")
        .and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .map(|id| AssetId::new(id.trim()))
                .collect()
        })
        .unwrap_or_default();
    if ids.is_empty() {
        return Err(
            "a voice is cloned from recordings of it: name at least one audio asset of \
                    the project — about a minute or two of clear speech in all. Nothing was \
                    sent."
                .to_owned(),
        );
    }
    let project = project_id(arguments)?;
    let stored = projects::open(&caller.toolbox.pool, caller.user, project)
        .await
        .map_err(|error| error.to_string())?;
    let mut tx = db::scoped(&caller.toolbox.pool, caller.user)
        .await
        .map_err(database)?;
    let mut recorded = Vec::with_capacity(ids.len());
    for id in &ids {
        let asset = stored.document.asset(id).ok_or_else(|| {
            format!("there is no asset {id} in this project, so nothing was sent")
        })?;
        let not_one = || {
            format!(
                "{id} is not an audio recording you uploaded, so nothing was sent — a voice is \
                 cloned only from somebody's own recorded speech"
            )
        };
        let hash = match (&asset.kind, &asset.sha256) {
            (AssetKind::Audio, Some(hash)) => hash,
            _ => return Err(not_one()),
        };
        let item: Option<(i64, String, String)> = sqlx::query_as(
            "SELECT id, extension, name FROM library_items
             WHERE sha256 = $1 AND kind = 'audio' AND brief_hash IS NULL",
        )
        .bind(hash)
        .fetch_optional(&mut *tx)
        .await
        .map_err(database)?;
        let (item, extension, name) = item.ok_or_else(not_one)?;
        recorded.push(Recorded {
            item,
            sha256: hash.clone(),
            extension,
            name,
        });
    }
    tx.commit().await.map_err(database)?;
    Ok(recorded)
}

/// Forget the caller's cloned voice `voice`: queued, free — the same job a
/// designed voice is forgotten by. Refused here unless it is one of theirs.
async fn forget(caller: &Caller<'_>, voice: &str) -> Result<Reply, String> {
    let toolbox = caller.toolbox;
    let mut tx = db::scoped(&toolbox.pool, caller.user)
        .await
        .map_err(database)?;
    let cloned = clones::cloned(&mut tx).await.map_err(database)?;
    let Some(cloned) = cloned.into_iter().find(|cloned| cloned.voice_id == voice) else {
        return Err(format!(
            "{voice} is not a voice you cloned, so nothing was deleted; voice_clone with list \
             shows the ones you did, by the voice_id to pass"
        ));
    };
    let payload = ForgetPayload {
        voice_id: voice.to_owned(),
    };
    let payload = serde_json::to_value(&payload).map_err(database)?;
    let job = jobs::enqueue(&mut tx, kinds::VOICE_FORGET, &payload)
        .await
        .map_err(database)?;
    tx.commit().await.map_err(database)?;
    toolbox.queue.announce(caller.user, &job);
    Ok(format!(
        "Forgetting {voice} (\"{}\") as job {} — free. Call jobs with job: {}; once it is done \
         the voice is gone from scorsese's ElevenLabs account and its slot is yours again. A \
         line already generated with it still plays; one not yet generated needs another \
         voice. The recordings stay in your library, so cloning them again makes a voice \
         like it.",
        cloned.name, job.id, job.id
    )
    .into())
}

/// Every voice the caller cloned.
async fn list(caller: &Caller<'_>) -> Result<Reply, String> {
    let mut tx = db::scoped(&caller.toolbox.pool, caller.user)
        .await
        .map_err(database)?;
    let cloned = clones::cloned(&mut tx).await.map_err(database)?;
    tx.commit().await.map_err(database)?;
    if cloned.is_empty() {
        return Ok(
            "You have not cloned any voices yet. Call voice_clone with a project, the \
                   audio assets to clone from, a name and consent."
                .into(),
        );
    }
    let lines: Vec<String> = cloned
        .iter()
        .map(|voice| {
            let from: Vec<String> = voice
                .recordings
                .iter()
                .map(|recorded| format!("library item {} ({})", recorded.item, recorded.name))
                .collect();
            format!(
                "{}  {}  (cloned)\n    from {}",
                voice.voice_id,
                voice.name,
                from.join(", ")
            )
        })
        .collect();
    Ok(lines.join("\n").into())
}
