//! `voice_design` on the hosted server (#572): the same brief, price and rules
//! as the stdio tool, paid from the user's credits, made by the job queue,
//! kept in the user's library and in their own record of designed voices
//! (`crate::designs` has why each lives where it does).
//!
//! A design quotes before it spends, exactly as `generate` does on the web
//! (#538): without `confirm` it sends nothing and answers with the price and a
//! token; with that token it reserves the price and queues the design as a
//! job, in one transaction. A design the user already has — every sample
//! still in their library — or one already on its way is answered for nothing,
//! with no token. The confirming call need not repeat the brief: the token's
//! own call is where it is read from, which is how a yes given in the
//! assistant's confirmation box (`assistant::quote`, which sends only the
//! token) designs what was quoted. `keep` and `list` spend nothing and take no
//! token, as locally.

mod keep;

use scorsese_core::Timestamp;
use scorsese_mcp::Reply;
use scorsese_providers::prices::dollars as cents_as_dollars;
use scorsese_providers::quote::{Charge, Item, Quote, Spend};
use scorsese_providers::voices::design::{Brief, PASSAGE, PROMPT, estimate};
use serde_json::{Value, json};

use super::{Caller, database, quotes};
use crate::credits::designs::Design;
use crate::credits::generations::{Request, start};
use crate::credits::{CreditError, dollars, from_cents, ledger, price};
use crate::db;
use crate::designs::DesignPayload;
use crate::jobs::{kinds, store as jobs};

/// How a client names it.
pub(crate) const NAME: &str = "voice_design";

/// What it does.
pub(crate) const DESCRIPTION: &str = "Design a new ElevenLabs voice from a description, for \
when no voice that voices lists is the one the video needs — paid from your credits, and it \
quotes before it spends. Called with prompt and text but no confirm it sends nothing: it \
answers with the price, what that takes from your credits, and a token. Show that quote to \
whoever is paying; only a second call with confirm set to the token designs, and only exactly \
what was quoted — the description and text need not be repeated. One design is billed once \
for the preview text and answers with three candidates to choose between, not three charges. \
Spending reserves the price and queues the design as a job; `jobs` says when it is done. Its \
three samples land in your library, where they can be played, and a design you already have \
is answered from there for nothing, with no token. A design ElevenLabs refuses costs nothing. \
Pass keep with a candidate id and name to turn one of them into a real voice_id a narration \
can name — free, no token, also a job whose result is the voice_id. Pass list to read back \
every voice you have designed. A kept voice lives in scorsese's ElevenLabs account, not in a \
project, so it is yours in every project, and the description and seed that made it are kept \
with it. Voice cloning from someone's recorded speech is not offered here in any form.";

/// Its arguments.
pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "project": {
                "type": "integer",
                "description": "The project the voice is for, by id, as project_list shows it. \
                                Optional: a designed voice is yours in every project; the \
                                project only says where the spending happened."
            },
            "prompt": {
                "type": "string",
                "description": format!(
                    "What the voice should be like, in a sentence: age, accent, pace, warmth, \
                     what it sounds like it is for. Between {} and {} characters. Describe a \
                     kind of person, never a named one — this designs a voice, it does not \
                     imitate anybody.",
                    PROMPT.start(), PROMPT.end()
                )
            },
            "text": {
                "type": "string",
                "description": format!(
                    "What the three candidates read aloud, between {} and {} characters. The \
                     only thing a design is billed for, so a longer passage is a better \
                     audition and a dearer one. Use a line the video would actually need.",
                    PASSAGE.start(), PASSAGE.end()
                )
            },
            "seed": {
                "type": "integer",
                "description": "Ask for the same candidates again, as far as the vendor \
                                manages it — best-effort. Worth passing: it is kept with the \
                                voice and is half of what makes a lost one worth asking for \
                                again."
            },
            "guidance": {
                "type": "number",
                "description": "How literally the candidates follow the description. Higher \
                                is more literal and less varied. Left out, the vendor chooses."
            },
            "confirm": {
                "type": "string",
                "description": "The token from this tool's own quote. Leave it out to be \
                                quoted — nothing is sent. Pass it back, after whoever is paying \
                                has agreed to the quoted price, to design exactly what was \
                                quoted. Good once, for fifteen minutes."
            },
            "keep": {
                "type": "string",
                "description": "A candidate id from one of your designs. Turns it into a real \
                                voice with a voice_id a narration can name. Costs nothing. \
                                Requires name."
            },
            "name": {
                "type": "string",
                "description": "What to call the kept voice — what it is recognisable by in \
                                every voice listing, so say what it is for."
            },
            "list": {
                "type": "boolean",
                "description": "Read back every voice you have designed and kept, with the \
                                description and seed that made each. Spends nothing."
            }
        }
    })
}

/// Design, keep or list.
pub(crate) async fn call(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    if arguments.get("list").and_then(Value::as_bool) == Some(true) {
        return keep::list(caller).await;
    }
    if let Some(chosen) = text(arguments, "keep") {
        let name = text(arguments, "name").ok_or(
            "keep needs a name: it is what the voice will be called in every voice listing, \
             and the only thing it is recognisable by there",
        )?;
        return keep::keep(caller, &chosen, &name).await;
    }
    let token = text(arguments, "confirm");
    let asked = match (&token, text(arguments, "prompt")) {
        (Some(token), None) => quoted(caller, token).await?,
        _ => arguments.clone(),
    };
    let brief = Brief::new(
        &text(&asked, "prompt").unwrap_or_default(),
        &text(&asked, "text").unwrap_or_default(),
        seed(&asked)?,
        asked.get("guidance").and_then(Value::as_f64),
    )
    .map_err(|error| error.to_string())?;
    let project = project(caller, &asked).await?;
    let digest = brief.digest();
    if let Some(said) = keep::already(caller, &digest).await? {
        return Ok(said.into());
    }
    let estimate = estimate(&brief.passage).map_err(|error| error.to_string())?;
    let quote = Quote {
        spend: Spend::VoiceDesign,
        items: vec![Item {
            subject: "design".to_owned(),
            says: estimate.says(),
            charge: Some(Charge {
                brief: digest.clone(),
                cents: estimate.cents,
            }),
        }],
    };
    let charged = credits(quote.cents());
    let now = Timestamp::unix_now().ok_or("the server's clock is before 1970")?;
    let pool = &caller.toolbox.pool;
    let mut tx = db::scoped(pool, caller.user).await.map_err(database)?;
    let Some(token) = token else {
        let issued = quotes::issue(&mut tx, &quote, now, caller.call)
            .await
            .map_err(database)?;
        let balance = ledger::balance(&mut tx).await.map_err(database)?;
        tx.commit().await.map_err(database)?;
        return Ok(format!(
            "design: {}\nAbout {} at ElevenLabs' published rate — {} from your credits with \
             scorsese's 10%. Our arithmetic, never a bill.\nYour balance is {}. Nothing has \
             been sent. To design, call voice_design again with confirm: \"{}\" — once whoever \
             is paying has agreed to {}. Good once, for fifteen minutes.",
            estimate.says(),
            cents_as_dollars(estimate.cents),
            dollars(charged),
            dollars(balance),
            issued.token,
            dollars(charged)
        )
        .into());
    };
    quotes::redeem(&mut tx, &token, &quote, now)
        .await
        .map_err(database)??;
    let payload = DesignPayload {
        brief: digest.clone(),
        prompt: brief.prompt.clone(),
        passage: brief.passage.clone(),
        seed: brief.seed,
        guidance: brief.guidance,
    };
    let payload = serde_json::to_value(&payload).map_err(database)?;
    let job = jobs::enqueue(&mut tx, kinds::VOICE_DESIGN, &payload)
        .await
        .map_err(database)?;
    let request = Request::Design(Design {
        project,
        tool_call: Some(caller.call),
        job: Some(job.id),
        prompt: &brief.prompt,
        passage: &brief.passage,
        seed: brief.seed,
        guidance: brief.guidance,
        brief_hash: &digest,
        estimated_cents: estimate.cents,
    });
    start(&mut tx, &request)
        .await
        .map_err(|error| match error {
            CreditError::Insufficient { .. } => error.to_string(),
            other => database(other),
        })?;
    tx.commit().await.map_err(database)?;
    caller.toolbox.queue.announce(caller.user, &job);
    Ok(format!(
        "Designing as job {}: {} reserved from your credits; a design ElevenLabs refuses gives \
         it back. Call jobs with job: {} to see it finish, then call voice_design again with \
         the same prompt and text — free — to see the three candidates and the library items \
         holding their samples.",
        job.id,
        dollars(charged),
        job.id
    )
    .into())
}

/// The arguments of the call that issued quote `token`, still unspent — what
/// a confirmation that names no brief designs.
async fn quoted(caller: &Caller<'_>, token: &str) -> Result<Value, String> {
    let mut tx = db::scoped(&caller.toolbox.pool, caller.user)
        .await
        .map_err(database)?;
    let asked = quotes::asked(&mut tx, token, NAME)
        .await
        .map_err(database)?;
    tx.commit().await.map_err(database)?;
    asked.ok_or_else(|| {
        "that token is not an unspent voice_design quote of yours; call voice_design without \
         confirm to be quoted again"
            .to_owned()
    })
}

/// The project an argument object names, if it names one — refused unless it
/// is the caller's.
async fn project(caller: &Caller<'_>, arguments: &Value) -> Result<Option<i64>, String> {
    if arguments.get("project").is_none_or(Value::is_null) {
        return Ok(None);
    }
    let id = super::project_id(arguments)?;
    let mut tx = db::scoped(&caller.toolbox.pool, caller.user)
        .await
        .map_err(database)?;
    let mine: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM projects WHERE id = $1)")
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(database)?;
    tx.commit().await.map_err(database)?;
    mine.then_some(Some(id))
        .ok_or_else(|| "there is no such project of yours".to_owned())
}

/// What `cents` of provider cost takes from a balance, in micro-dollars.
fn credits(cents: u64) -> i64 {
    price(from_cents(cents))
}

/// A string argument, blank counting as absent.
fn text(arguments: &Value, name: &str) -> Option<String> {
    arguments
        .get(name)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

/// The seed, refused rather than rounded when it is not a whole number.
fn seed(arguments: &Value) -> Result<Option<u32>, String> {
    match arguments.get("seed") {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_u64()
            .and_then(|value| u32::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| format!("seed: {value} is not a whole number a seed can be")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::designs;

    #[test]
    fn a_seed_is_a_whole_number_or_nothing() {
        assert_eq!(seed(&json!({})), Ok(None));
        assert_eq!(seed(&json!({ "seed": 7 })), Ok(Some(7)));
        assert!(seed(&json!({ "seed": 1.5 })).is_err());
        assert!(seed(&json!({ "seed": -1 })).is_err());
    }

    #[test]
    fn blank_text_is_absent() {
        assert_eq!(text(&json!({ "keep": "  " }), "keep"), None);
        assert_eq!(
            text(&json!({ "keep": " a " }), "keep"),
            Some("a".to_owned())
        );
    }

    #[test]
    fn a_design_sample_has_a_hash_the_library_accepts() {
        let hash = designs::sample_hash(&"0".repeat(64), 2);
        assert_eq!(hash.len(), 64);
        assert!(
            hash.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        );
        assert_ne!(hash, designs::sample_hash(&"0".repeat(64), 3));
    }
}
