//! Vendors that never spend a cent: a Veo, a Gemini — drawing now or in a
//! batch — and an ElevenLabs —
//! speech, music and voice design — answering from files ffmpeg made, and
//! counting how often they were asked. Veo is [`video`], the Gemini half [`image`],
//! ElevenLabs' speech [`speech`] and its music [`music`], and the local
//! Whisper [`whisper`] (#1549).

mod image;
mod music;
mod speech;
mod video;
mod whisper;

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use scorsese_render::testing::generate;
use scorsese_server::generations::{Ear, Image, Music, Speech, Studio, Timing, Vendors, Video};
use scorsese_server::http::AppState;
use scorsese_server::jobs::elevenlabs::ElevenLabs;
use scorsese_server::jobs::{self, kinds};
use sqlx::postgres::PgPool;
use tokio::sync::watch;

use super::common;

/// What the mock vendors answer, and how often they were asked.
#[derive(Clone, Default)]
pub(super) struct Mock {
    /// A refusal to give when polled, instead of the video.
    pub(super) refuse: Option<String>,
    /// Why Veo no longer knows the ticket, answered when polled (#1196).
    pub(super) lose: Option<String>,
    /// Why a batch stops without drawing, instead of finishing.
    pub(super) stop: Option<String>,
    pub(super) ordered: Arc<AtomicUsize>,
    pub(super) spoken: Arc<AtomicUsize>,
    pub(super) composed: Arc<AtomicUsize>,
    pub(super) submitted: Arc<AtomicUsize>,
    pub(super) drawn: Arc<AtomicUsize>,
    pub(super) designed: Arc<AtomicUsize>,
    pub(super) kept: Arc<AtomicUsize>,
    pub(super) forgotten: Arc<AtomicUsize>,
    pub(super) cloned: Arc<AtomicUsize>,
    /// Why ElevenLabs will not delete a voice, instead of deleting it.
    pub(super) refuse_forget: Option<String>,
    /// How often the Whisper was asked to hear something (#1549).
    pub(super) heard: Arc<AtomicUsize>,
}

impl Mock {
    pub(super) fn spoken(&self) -> usize {
        self.spoken.load(Ordering::SeqCst)
    }

    pub(super) fn submitted(&self) -> usize {
        self.submitted.load(Ordering::SeqCst)
    }

    pub(super) fn drawn(&self) -> usize {
        self.drawn.load(Ordering::SeqCst)
    }

    pub(super) fn ordered(&self) -> usize {
        self.ordered.load(Ordering::SeqCst)
    }
}

impl Vendors for Mock {
    fn video(&self) -> Result<Video, String> {
        Ok(Box::new(self.clone()))
    }

    fn speech(&self) -> Result<Speech, String> {
        Ok(Box::new(self.clone()))
    }

    fn music(&self) -> Result<Music, String> {
        Ok(Box::new(self.clone()))
    }

    fn image(&self) -> Result<Image, String> {
        Ok(Box::new(self.clone()))
    }

    fn studio(&self) -> Result<Studio, String> {
        Ok(Box::new(self.clone()))
    }

    fn whisper(&self) -> Result<Ear, String> {
        Ok(Box::new(self.clone()))
    }
}

/// A file ffmpeg makes from `inputs`, as bytes.
pub(super) fn made(name: &str, inputs: &[&str]) -> Vec<u8> {
    let file = common::scratch::dir("vendor").join(name);
    generate(&common::tools(), &file, inputs);
    std::fs::read(file).expect("the fixture reads")
}

/// A server whose job worker reaches `mock`: its address, its state, and
/// what keeps the worker running — dropping it stops the worker.
pub(super) async fn serve(
    pool: &PgPool,
    mock: &Mock,
) -> (SocketAddr, AppState, watch::Sender<bool>) {
    serve_on(pool, mock, ElevenLabs::default()).await
}

/// [`serve`], on an ElevenLabs plan that allows what `plan` says.
pub(super) async fn serve_on(
    pool: &PgPool,
    mock: &Mock,
    plan: ElevenLabs,
) -> (SocketAddr, AppState, watch::Sender<bool>) {
    let files = common::files("mcp-paid");
    let (address, state) = common::serve_with(pool.clone(), files.clone()).await;
    let timing = Timing {
        poll_every: Duration::from_millis(20),
        patience: Duration::from_secs(20),
        batch_every: Duration::from_millis(20),
        batch_patience: Duration::from_secs(20),
    };
    let registry = kinds::with_vendors(&files, Arc::new(mock.clone()), timing, plan);
    let (stop, stopping) = watch::channel(false);
    let worker = tokio::spawn(jobs::work(
        state.pool.clone(),
        registry,
        state.jobs.clone(),
        stopping,
    ));
    common::release_at_teardown(pool, &state.pool, [worker.abort_handle()]);
    (address, state, stop)
}

/// Wait until every job of `user`'s has finished; their states.
pub(super) async fn settled(state: &AppState, user: scorsese_server::db::UserId) -> Vec<String> {
    for _ in 0..400 {
        let all = jobs::store::list(&state.pool, user)
            .await
            .expect("the jobs list");
        let busy = all.iter().any(|job| {
            matches!(job.state, jobs::State::Waiting | jobs::State::Running)
                && job.kind != "thumbnail"
        });
        if !busy {
            return all
                .iter()
                .filter(|job| job.kind != "thumbnail")
                .map(|job| format!("{:?}", job.state).to_lowercase())
                .collect();
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("the jobs never finished");
}
