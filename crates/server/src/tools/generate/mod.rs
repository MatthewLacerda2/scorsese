//! `generate` on the hosted server: the same quote, paid from the user's
//! credits, made by the job queue.
//!
//! The quote is `scorsese_providers::quote::generation` over the project laid
//! out with the user's files **and every generation they already have**
//! ([`lay_out`](super::lay_out)), so a brief made before — in this project or
//! another of theirs — is priced at nothing, exactly as a local project's
//! `generated/` file is. A brief whose job is still on its way is free too:
//! it is being paid for already. Then the rule every paid tool follows
//! (#538): a call without `confirm` spends nothing and answers with the quote
//! and a token; only a call with that token, while what would be paid for is
//! still what was quoted, spends ([`quotes`](super::quotes)).
//!
//! Spending is not calling a provider. Each charged brief is **reserved** from
//! the user's credits and **queued** as a job in one transaction ([`spend`]),
//! all or none — a balance that covers three shots of five queues none — and
//! the jobs do the rest (`crate::generations`): the provider, the library, the
//! charge or the release, and the project. So the call answers at once, with
//! the jobs to ask `jobs` about.

mod spend;

use scorsese_core::{GenerationState, Timestamp};
use scorsese_mcp::Reply;
use scorsese_providers::prices::dollars as cents_as_dollars;
use scorsese_providers::quote::{Item, Quote, generation};
use serde_json::{Value, json};

use super::surface::project_property;
use super::{Caller, database, lay_out, project_id, quotes};
use crate::credits::{dollars, from_cents, ledger, price};
use crate::db;
use crate::generations::adopt;
use crate::projects::{self, ProjectError};

/// How a client names it.
pub(crate) const NAME: &str = "generate";

/// What it does.
pub(crate) const DESCRIPTION: &str = "Realise the sketched briefs — Veo shots and ElevenLabs \
lines — paid from your credits, and it quotes before it spends. Called without confirm it sends \
nothing: it answers with what each generated_video and generated_audio brief would cost and \
what that takes from your credits, and a token. Show that quote to whoever is paying; only a \
second call with confirm set to the token spends, and only on exactly the briefs quoted — edit \
one in between and the call is refused and must be quoted again. Spending reserves the price \
and queues each brief as a job; the call answers at once with the job ids, and `jobs` says \
when each is done. A finished generation lands in your library and in the project on its own. \
A brief the provider refuses costs nothing; one that works is charged whether or not it is \
kept. A brief you already generated, in this project or another, is never paid for again, \
and a line with no voice chosen yet is reported and skipped. Every price is our own \
arithmetic over published rates.";

/// Its arguments.
pub(crate) fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "project": project_property(),
            "confirm": {
                "type": "string",
                "description": "The token from this tool's own quote. Leave it out to be \
                                quoted — nothing is sent. Pass it back, after whoever is \
                                paying has agreed to the quoted price, to spend exactly what \
                                was quoted. Good once, for fifteen minutes; if a brief changes \
                                in between, the call is refused and a new quote is needed."
            }
        },
        "required": ["project"]
    })
}

/// Quote, or spend a confirmed quote.
pub(crate) async fn call(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    let id = project_id(arguments)?;
    let toolbox = caller.toolbox;
    let mut stored = open(caller, id).await?;
    // A brief already generated — here or in another of the user's projects —
    // is brought in first, so the document says so and the quote agrees.
    let waiting =
        stored.document.assets.iter().any(|asset| {
            asset.kind.is_prompted() && asset.state != Some(GenerationState::Generated)
        });
    if waiting {
        let (pool, storage, tools) = (&toolbox.pool, &toolbox.storage, &toolbox.tools);
        if !adopt(pool, storage, tools, caller.user, id)
            .await?
            .is_empty()
        {
            stored = open(caller, id).await?;
        }
    }
    let folder = lay_out(
        &toolbox.pool,
        &toolbox.storage,
        caller.user,
        &stored.document,
    )
    .await?;
    let (root, document) = (folder.root().to_path_buf(), stored.document.clone());
    let (mut quote, charged) = tokio::task::spawn_blocking(move || {
        let quote = generation(&document, &root).map_err(|error| error.to_string())?;
        let charged = spend::charged(&quote, &document, &root);
        Ok::<_, String>((quote, charged))
    })
    .await
    .map_err(|_| "quoting crashed on the server; that is a bug".to_owned())??;
    drop(folder);
    on_its_way(caller, &mut quote).await?;
    let still_charged = |brief: &str| {
        quote.items.iter().any(|item| {
            item.charge
                .as_ref()
                .is_some_and(|charge| charge.brief == brief)
        })
    };
    let charged: Vec<_> = charged
        .into_iter()
        .filter(|one| still_charged(one.brief()))
        .collect();

    let mut lines = said(&quote);
    if quote.is_free() {
        lines.push("Nothing to pay for, so nothing was sent.".to_owned());
        return Ok(lines.join("\n").into());
    }
    let now = Timestamp::unix_now().ok_or("the server's clock is before 1970")?;
    let pool = &toolbox.pool;
    let mut tx = db::scoped(pool, caller.user).await.map_err(database)?;
    let Some(token) = arguments.get("confirm").and_then(Value::as_str) else {
        let issued = quotes::issue(&mut tx, &quote, now)
            .await
            .map_err(database)?;
        let balance = ledger::balance(&mut tx).await.map_err(database)?;
        tx.commit().await.map_err(database)?;
        lines.push(format!(
            "Your balance is {}. Nothing has been sent. To spend this, call generate again \
             with the same project and confirm: \"{}\" — once whoever is paying has agreed to \
             {}. Good once, for fifteen minutes, for exactly these briefs; if one changes, \
             quote again.",
            dollars(balance),
            issued.token,
            dollars(credits(issued.cents))
        ));
        return Ok(lines.join("\n").into());
    };
    let redeemed = quotes::redeem(&mut tx, token.trim(), &quote, now)
        .await
        .map_err(database)?;
    tx.commit().await.map_err(database)?;
    redeemed?;

    let queued = spend::spend(caller, id, &stored.document, &charged).await?;
    let mut lines: Vec<String> = queued
        .iter()
        .map(|(asset, job)| format!("{asset}: queued as job {}", job.id))
        .collect();
    lines.push(format!(
        "{} reserved from your credits; a brief the provider refuses gives its share back. \
         Call jobs to see each one finish — each lands in your library and in the project \
         on its own.",
        dollars(credits(quote.cents()))
    ));
    Ok(lines.join("\n").into())
}

/// The caller's project `id`.
async fn open(caller: &Caller<'_>, id: i64) -> Result<projects::Stored, String> {
    projects::open(&caller.toolbox.pool, caller.user, id)
        .await
        .map_err(|error| match error {
            ProjectError::NotFound => "there is no such project of yours".to_owned(),
            other => database(other),
        })
}

/// Mark as free every charged brief a job of the caller's is already making:
/// it is paid for, and a second yes must not pay for it again.
async fn on_its_way(caller: &Caller<'_>, quote: &mut Quote) -> Result<(), String> {
    let briefs: Vec<String> = quote
        .items
        .iter()
        .filter_map(|item| Some(item.charge.as_ref()?.brief.clone()))
        .collect();
    if briefs.is_empty() {
        return Ok(());
    }
    let mut tx = db::scoped(&caller.toolbox.pool, caller.user)
        .await
        .map_err(database)?;
    let running: Vec<(i64, String)> = sqlx::query_as(
        "SELECT id, payload->>'brief' FROM jobs
         WHERE kind IN ('veo_shot', 'spoken_line') AND state IN ('waiting', 'running', 'stuck')
           AND payload->>'brief' = ANY($1)",
    )
    .bind(&briefs)
    .fetch_all(&mut *tx)
    .await
    .map_err(database)?;
    tx.commit().await.map_err(database)?;
    for item in &mut quote.items {
        let job = item.charge.as_ref().and_then(|charge| {
            running
                .iter()
                .find(|(_, brief)| *brief == charge.brief)
                .map(|(job, _)| *job)
        });
        if let Some(job) = job {
            *item = Item {
                subject: item.subject.clone(),
                says: format!("already on its way as job {job} — paid for, nothing more to pay"),
                charge: None,
            };
        }
    }
    Ok(())
}

/// The quote as lines: one per item, then the total, at the providers' rates
/// and as it comes out of the user's credits.
fn said(quote: &Quote) -> Vec<String> {
    let mut lines: Vec<String> = quote
        .items
        .iter()
        .map(|item| format!("{}: {}", item.subject, item.says))
        .collect();
    if quote.items.is_empty() {
        lines.push("No generated_video or generated_audio assets in this project.".to_owned());
    } else if !quote.is_free() {
        lines.push(format!(
            "About {} at the providers' published rates — {} from your credits with \
             scorsese's 10%. Our arithmetic, never a bill.",
            cents_as_dollars(quote.cents()),
            dollars(credits(quote.cents()))
        ));
    }
    lines
}

/// What `cents` of provider cost takes from a balance, in micro-dollars.
fn credits(cents: u64) -> i64 {
    price(from_cents(cents))
}
