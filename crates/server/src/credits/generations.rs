//! The audit of every paid generation: `veo_generations`,
//! `image_generations`, `speech_generations`, `music_generations` (#1264),
//! `sound_generations` (#1265) and `voice_designs` (#572, [`designs`]), each row bound to the reservation
//! that pays for it.
//!
//! For debugging, disputes and improving the platform — and for the user, who
//! can read every charge of theirs in the history. A generation's life is
//! [`start`] (priced, reserved, recorded, or refused with nothing written),
//! then for a shot or a batched still [`keep_ticket`] the moment Google
//! accepts, then [`finish`]
//! with what the provider said. The job handlers that call these are
//! `crate::generations` (#539); web `generate` starts one in the transaction
//! that enqueues its job, and the job finds what pays for it with
//! [`for_job`].

use super::designs::{self, Design};
use super::ledger::{self, Funds, Link, Reservation};
use super::lines;
use super::shots;
use super::sounds;
use super::stills;
use super::tracks;
use super::{CreditError, Product, charge_for, from_cents};
use crate::db::Tx;

pub use super::lines::Line;
pub use super::shots::Shot;
pub use super::sounds::Sound;
pub use super::stills::Still;
pub use super::tracks::Track;

/// Which audit row a generation is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Generation {
    /// A row of `veo_generations`.
    Shot(i64),
    /// A row of `speech_generations`.
    Line(i64),
    /// A row of `image_generations`.
    Still(i64),
    /// A row of `voice_designs`.
    Design(i64),
    /// A row of `music_generations`.
    Track(i64),
    /// A row of `sound_generations`.
    Sound(i64),
}

/// A generation recorded and paid for, waiting on its provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paid {
    /// Its audit row.
    pub generation: Generation,
    /// What it holds of the user's balance.
    pub reservation: Reservation,
}

/// What to record before calling a provider.
#[derive(Debug, Clone)]
pub enum Request<'a> {
    /// A Veo shot.
    Shot(Shot<'a>),
    /// A spoken line.
    Line(Line<'a>),
    /// A generated still.
    Still(Still<'a>),
    /// A voice design.
    Design(Design<'a>),
    /// A music track.
    Track(Track<'a>),
    /// A sound effect.
    Sound(Sound<'a>),
}

/// Price, reserve and record a generation — or refuse, writing nothing, when
/// the balance cannot cover it. Call it before anything reaches the provider.
pub async fn start(tx: &mut Tx, request: &Request<'_>) -> Result<Paid, CreditError> {
    let (product, cents, project) = match request {
        Request::Shot(shot) => (Product::VeoShot, shot.estimated_cents, shot.project),
        Request::Line(line) => (Product::SpokenLine, line.estimated_cents, line.project),
        Request::Still(still) => (Product::Still, still.estimated_cents, still.project),
        Request::Design(design) => (Product::VoiceDesign, design.estimated_cents, design.project),
        Request::Track(track) => (Product::MusicTrack, track.estimated_cents, track.project),
        Request::Sound(sound) => (Product::SoundEffect, sound.estimated_cents, sound.project),
    };
    let charged = charge_for(product, cents);
    let funds = if product == Product::VeoShot {
        Funds::PaidOnly
    } else {
        Funds::Any
    };
    ledger::cover(tx, charged, funds).await?;
    let cost = i64::try_from(from_cents(cents)).unwrap_or(i64::MAX);
    let (generation, memo, link) = match request {
        Request::Shot(shot) => {
            let id = shots::insert(tx, shot, cost).await?;
            let memo = shots::memo(shot);
            let link = Link {
                project,
                veo: Some(id),
                ..Link::default()
            };
            (Generation::Shot(id), memo, link)
        }
        Request::Line(line) => {
            let id = lines::insert(tx, line, cost).await?;
            let memo = lines::memo(line);
            let link = Link {
                project,
                speech: Some(id),
                ..Link::default()
            };
            (Generation::Line(id), memo, link)
        }
        Request::Still(still) => {
            let id = stills::insert(tx, still, cost).await?;
            let memo = stills::memo(still);
            let link = Link {
                project,
                image: Some(id),
                ..Link::default()
            };
            (Generation::Still(id), memo, link)
        }
        Request::Design(design) => {
            let id = designs::insert(tx, design, cost).await?;
            let link = Link {
                project,
                design: Some(id),
                ..Link::default()
            };
            (Generation::Design(id), designs::memo(design), link)
        }
        Request::Track(track) => {
            let id = tracks::insert(tx, track, cost).await?;
            let memo = tracks::memo(track);
            let link = Link {
                project,
                music: Some(id),
                ..Link::default()
            };
            (Generation::Track(id), memo, link)
        }
        Request::Sound(sound) => {
            let id = sounds::insert(tx, sound, cost).await?;
            let link = Link {
                project,
                sound: Some(id),
                ..Link::default()
            };
            (Generation::Sound(id), sounds::memo(sound), link)
        }
    };
    let reservation = ledger::reserve(tx, charged, &memo, link).await?;
    Ok(Paid {
        generation,
        reservation,
    })
}

/// Record Google's ticket on a shot's or a batched still's audit row — its
/// operation, or its batch job's name. The job's own row keeps it too
/// (`jobs::Context::keep_ticket`); this one outlives the job. Nothing else
/// has a ticket, so nothing is written for it.
pub async fn keep_ticket(
    tx: &mut Tx,
    generation: Generation,
    ticket: &str,
) -> Result<(), sqlx::Error> {
    let (query, id) = match generation {
        Generation::Shot(id) => ("UPDATE veo_generations SET ticket = $2 WHERE id = $1", id),
        Generation::Still(id) => ("UPDATE image_generations SET ticket = $2 WHERE id = $1", id),
        Generation::Line(_)
        | Generation::Design(_)
        | Generation::Track(_)
        | Generation::Sound(_) => return Ok(()),
    };
    sqlx::query(query)
        .bind(id)
        .bind(ticket)
        .execute(&mut **tx)
        .await
        .map(drop)
}

/// The generation job `job` pays for, while its reservation is still open —
/// `None` once it has been settled, or for a job nothing paid for.
///
/// What a generation job reads when it starts: the job is enqueued in the
/// transaction that reserves for it, so the audit row names the job, and a
/// job run again after a crash that had already settled finds nothing to
/// settle twice.
pub async fn for_job(tx: &mut Tx, job: i64) -> Result<Option<Paid>, sqlx::Error> {
    let row: Option<Reserved> = sqlx::query_as(
        "SELECT e.veo_generation_id, e.speech_generation_id, e.image_generation_id,
                e.voice_design_id, e.music_generation_id, e.sound_generation_id, e.id,
                -e.amount_micros
         FROM credit_entries e
         LEFT JOIN veo_generations v ON v.id = e.veo_generation_id
         LEFT JOIN speech_generations s ON s.id = e.speech_generation_id
         LEFT JOIN image_generations i ON i.id = e.image_generation_id
         LEFT JOIN voice_designs d ON d.id = e.voice_design_id
         LEFT JOIN music_generations m ON m.id = e.music_generation_id
         LEFT JOIN sound_generations x ON x.id = e.sound_generation_id
         WHERE e.kind = 'reservation'
           AND (v.job_id = $1 OR s.job_id = $1 OR i.job_id = $1 OR d.job_id = $1
                OR m.job_id = $1 OR x.job_id = $1)
           AND NOT EXISTS (SELECT 1 FROM credit_entries r
                           WHERE r.settles = e.id AND r.kind = 'release')",
    )
    .bind(job)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(
        row.and_then(|(shot, line, still, design, track, sound, entry, micros)| {
            let generation = match (shot, line, still, design, track, sound) {
                (Some(id), ..) => Generation::Shot(id),
                (None, Some(id), ..) => Generation::Line(id),
                (None, None, Some(id), ..) => Generation::Still(id),
                (None, None, None, Some(id), ..) => Generation::Design(id),
                (None, None, None, None, Some(id), _) => Generation::Track(id),
                (None, None, None, None, None, Some(id)) => Generation::Sound(id),
                (None, None, None, None, None, None) => return None,
            };
            Some(Paid {
                generation,
                reservation: Reservation { entry, micros },
            })
        }),
    )
}

/// An open reservation's row: the shot, line, still, design, track or sound
/// it pays for — one of the six — then its entry and what it holds.
type Reserved = (
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    i64,
    i64,
);

/// How the provider answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answer {
    /// It worked, and produced this library item (`Library::keep_generated`).
    Worked(Option<i64>),
    /// The provider failed, and this is why — so it is free.
    Failed(String),
    /// The vendor completed the call, and bills it, but made nothing: this is
    /// what it said instead. Charged as quoted, since the member accepted the
    /// quote and the vendor bills the call (#1646).
    Unmade(String),
}

/// Settle a generation: charged if it worked or the vendor billed a call that
/// made nothing, released if the provider failed.
pub async fn finish(tx: &mut Tx, paid: Paid, answer: &Answer) -> Result<(), CreditError> {
    let (state, item, error) = match answer {
        Answer::Worked(item) => {
            ledger::charge(tx, paid.reservation, paid.reservation.micros).await?;
            ("generated", *item, None)
        }
        Answer::Unmade(why) => {
            ledger::charge(tx, paid.reservation, paid.reservation.micros).await?;
            ("failed", None, Some(why.as_str()))
        }
        Answer::Failed(why) => {
            ledger::release(tx, paid.reservation).await?;
            ("failed", None, Some(why.as_str()))
        }
    };
    let (query, id) = match paid.generation {
        Generation::Shot(id) => (
            "UPDATE veo_generations SET state = $2, library_item_id = $3, error = $4,
                 finished_at = now() WHERE id = $1",
            id,
        ),
        Generation::Line(id) => (
            "UPDATE speech_generations SET state = $2, library_item_id = $3, error = $4,
                 finished_at = now() WHERE id = $1",
            id,
        ),
        Generation::Still(id) => (
            "UPDATE image_generations SET state = $2, library_item_id = $3, error = $4,
                 finished_at = now() WHERE id = $1",
            id,
        ),
        Generation::Design(id) => (
            "UPDATE voice_designs SET state = $2, library_item_id = $3, error = $4,
                 finished_at = now() WHERE id = $1",
            id,
        ),
        Generation::Track(id) => (
            "UPDATE music_generations SET state = $2, library_item_id = $3, error = $4,
                 finished_at = now() WHERE id = $1",
            id,
        ),
        Generation::Sound(id) => (
            "UPDATE sound_generations SET state = $2, library_item_id = $3, error = $4,
                 finished_at = now() WHERE id = $1",
            id,
        ),
    };
    sqlx::query(query)
        .bind(id)
        .bind(state)
        .bind(item)
        .bind(error)
        .execute(&mut **tx)
        .await?;
    Ok(())
}
