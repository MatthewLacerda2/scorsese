//! Veo, mocked: a ticket, then the shot or the refusal the mock was given.

use std::sync::atomic::Ordering;

use scorsese_providers::video::{self, Progress, ProviderError, Ready, Ticket, VideoProvider};

use super::{Mock, made};

impl VideoProvider for Mock {
    fn submit(&self, _: &video::Brief) -> Result<Ticket, ProviderError> {
        self.submitted.fetch_add(1, Ordering::SeqCst);
        Ok(Ticket("operations/mock-1".into()))
    }

    fn poll(&self, _: &Ticket) -> Result<Progress, ProviderError> {
        if let Some(why) = &self.lose {
            return Err(ProviderError::lost("mock video", why));
        }
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
