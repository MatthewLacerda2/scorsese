//! Vendors that never spend a cent: a Veo, a Gemini and an ElevenLabs —
//! speech and voice design — answering from files ffmpeg made, and counting
//! how often they were asked.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use scorsese_providers::image::{self, ImageProvider};
use scorsese_providers::speech::{self, SpeechProvider};
use scorsese_providers::video::{self, Progress, ProviderError, Ready, Ticket, VideoProvider};
use scorsese_server::generations::{Image, Speech, Studio, Timing, Vendors, Video};
use scorsese_server::http::AppState;
use scorsese_server::jobs::{self, kinds};
use sqlx::postgres::PgPool;
use tokio::sync::watch;

use super::common;

/// What the mock vendors answer, and how often they were asked.
#[derive(Clone, Default)]
pub(super) struct Mock {
    /// A refusal to give when polled, instead of the video.
    pub(super) refuse: Option<String>,
    pub(super) spoken: Arc<AtomicUsize>,
    pub(super) submitted: Arc<AtomicUsize>,
    pub(super) drawn: Arc<AtomicUsize>,
    pub(super) designed: Arc<AtomicUsize>,
    pub(super) kept: Arc<AtomicUsize>,
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
}

impl Vendors for Mock {
    fn video(&self) -> Result<Video, String> {
        Ok(Box::new(self.clone()))
    }

    fn speech(&self) -> Result<Speech, String> {
        Ok(Box::new(self.clone()))
    }

    fn image(&self) -> Result<Image, String> {
        Ok(Box::new(self.clone()))
    }

    fn studio(&self) -> Result<Studio, String> {
        Ok(Box::new(self.clone()))
    }
}

impl ImageProvider for Mock {
    fn draw(&self, _: &image::Brief) -> Result<Vec<u8>, ProviderError> {
        self.drawn.fetch_add(1, Ordering::SeqCst);
        Ok(made(
            "still.jpg",
            &["-f", "lavfi", "-i", "testsrc=s=64x36", "-frames:v", "1"],
        ))
    }

    fn name(&self) -> &'static str {
        "mock image"
    }
}

impl SpeechProvider for Mock {
    fn speak(&self, _: &speech::Brief) -> Result<speech::Spoken, ProviderError> {
        self.spoken.fetch_add(1, Ordering::SeqCst);
        Ok(made("line.mp3", &["-f", "lavfi", "-i", "sine=duration=1"]).into())
    }

    fn name(&self) -> &'static str {
        "mock speech"
    }
}

impl VideoProvider for Mock {
    fn submit(&self, _: &video::Brief) -> Result<Ticket, ProviderError> {
        self.submitted.fetch_add(1, Ordering::SeqCst);
        Ok(Ticket("operations/mock-1".into()))
    }

    fn poll(&self, _: &Ticket) -> Result<Progress, ProviderError> {
        Ok(match &self.refuse {
            Some(why) => Progress::Failed(why.clone()),
            None => Progress::Ready(Ready("mock".into())),
        })
    }

    fn fetch(&self, _: &Ready) -> Result<Vec<u8>, ProviderError> {
        Ok(made(
            "shot.mp4",
            &[
                "-f",
                "lavfi",
                "-i",
                "testsrc=s=64x64:d=1:r=30",
                "-pix_fmt",
                "yuv420p",
            ],
        ))
    }

    fn name(&self) -> &'static str {
        "mock video"
    }
}

/// A file ffmpeg makes from `inputs`, as bytes.
pub(super) fn made(name: &str, inputs: &[&str]) -> Vec<u8> {
    let file = common::scratch("vendor").join(name);
    let output = common::tools()
        .ffmpeg()
        .args(["-v", "error", "-y"])
        .args(inputs)
        .arg(&file)
        .output()
        .expect("ffmpeg runs");
    assert!(output.status.success(), "{output:?}");
    std::fs::read(file).expect("the fixture reads")
}

/// A server whose job worker reaches `mock`: its address, its state, and
/// what keeps the worker running — dropping it stops the worker.
pub(super) async fn serve(
    pool: &PgPool,
    mock: &Mock,
) -> (SocketAddr, AppState, watch::Sender<bool>) {
    let files = common::files("mcp-paid");
    let (address, state) = common::serve_with(pool.clone(), files.clone()).await;
    let timing = Timing {
        poll_every: Duration::from_millis(20),
        patience: Duration::from_secs(20),
    };
    let registry = kinds::with_vendors(&files, Arc::new(mock.clone()), timing);
    let (stop, stopping) = watch::channel(false);
    tokio::spawn(jobs::work(
        state.pool.clone(),
        registry,
        state.jobs.clone(),
        stopping,
    ));
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
