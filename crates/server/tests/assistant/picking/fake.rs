//! A stock library that answers from memory and remembers what it was asked
//! to download.

use std::io::Write;
use std::sync::{Arc, Mutex};

use scorsese_providers::stock::{Candidate, Library, Medium, Page, Query, Rendition, StockError};

use super::common;

/// The Lottie every animation of the fake downloads as: the hand-made stand-in
/// `scorsese-render`'s tests play.
const WAVE: &str = include_str!("../../../../render/tests/fixtures/lottie/wave.json");

/// The animations a search for a Lottie lists.
pub(crate) const LOTTIES: [u64; 2] = [7, 8];

/// Images 1 to 3, each a picture of a sunrise in a colour of its own, and
/// [`LOTTIES`], each the same small animation; it remembers what it was
/// asked to download.
pub(crate) struct Fake {
    pngs: Vec<Vec<u8>>,
    downloaded: Mutex<Vec<String>>,
}

impl Fake {
    pub(crate) fn new() -> Arc<Self> {
        let scratch = common::scratch("picking");
        let pngs = ["red", "orange", "yellow"].map(|colour| {
            let file = scratch.join(format!("{colour}.png"));
            let made = common::tools()
                .ffmpeg()
                .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
                .arg(format!("color=c={colour}:s=64x36"))
                .args(["-frames:v", "1"])
                .arg(&file)
                .output()
                .expect("ffmpeg runs");
            assert!(made.status.success(), "{made:?}");
            std::fs::read(&file).expect("the picture was made")
        });
        Arc::new(Self {
            pngs: pngs.into(),
            downloaded: Mutex::default(),
        })
    }

    /// Every URL downloaded, in order.
    pub(crate) fn downloaded(&self) -> Vec<String> {
        self.downloaded
            .lock()
            .expect("the test setup holds")
            .clone()
    }

    /// The files downloaded to be imported, in order — every download but a
    /// search's previews.
    pub(crate) fn imported(&self) -> Vec<String> {
        let downloaded = self.downloaded();
        downloaded
            .into_iter()
            .filter(|url| !url.ends_with("_640.jpg") && !url.ends_with(".png.jpg"))
            .collect()
    }
}

/// Image `id`, as a search lists it.
fn picture(id: u64) -> Candidate {
    Candidate {
        medium: Medium::Image,
        id,
        title: String::new(),
        style: "photo".into(),
        tags: vec!["sunrise".into()],
        seconds: None,
        fps: None,
        author: "someone".into(),
        page_url: format!("https://pixabay.com/photos/id-{id}/"),
        preview_url: format!("https://cdn.example.invalid/{id}_640.jpg"),
        motion_url: None,
        animated_url: None,
        ai_generated: false,
        renditions: vec![Rendition {
            name: "large".into(),
            url: format!("https://cdn.example.invalid/{id}_1920.png"),
            width: 1920,
            height: 1080,
            bytes: None,
        }],
    }
}

/// Animation `id`, as a search lists it.
fn animation(id: u64) -> Candidate {
    Candidate {
        medium: Medium::Lottie,
        id,
        title: format!("Wave {id}"),
        style: String::new(),
        tags: vec!["wave".into()],
        seconds: Some(2),
        fps: Some(30),
        author: "someone".into(),
        page_url: format!("https://lottiefiles.com/free-animation/wave-{id}"),
        preview_url: format!("https://cdn.example.invalid/{id}.png.jpg"),
        motion_url: Some(format!("https://cdn.example.invalid/{id}.mp4")),
        animated_url: Some(format!("https://cdn.example.invalid/{id}.gif")),
        ai_generated: false,
        renditions: vec![Rendition {
            name: "json".into(),
            url: format!("https://cdn.example.invalid/{id}.json"),
            width: 64,
            height: 64,
            bytes: None,
        }],
    }
}

impl Library for Fake {
    fn name(&self) -> &'static str {
        "Fake"
    }

    fn page(&self, query: &Query, _: u32) -> Result<Page, StockError> {
        let candidates: Vec<Candidate> = match query.medium {
            Medium::Lottie => LOTTIES.into_iter().map(animation).collect(),
            _ => (1..=3).map(picture).collect(),
        };
        let total = candidates.len() as u64;
        Ok(Page { candidates, total })
    }

    fn one(&self, medium: Medium, id: u64) -> Result<Option<Candidate>, StockError> {
        Ok(match medium {
            Medium::Lottie => LOTTIES.contains(&id).then(|| animation(id)),
            _ => (1..=3).contains(&id).then(|| picture(id)),
        })
    }

    fn download(&self, url: &str, _: u64, to: &mut dyn Write) -> Result<u64, StockError> {
        let mut downloaded = self.downloaded.lock().expect("the test setup holds");
        downloaded.push(url.to_owned());
        if url.ends_with(".json") {
            to.write_all(WAVE.as_bytes()).expect("the test setup holds");
            return Ok(WAVE.len() as u64);
        }
        let id = (1..=3).find(|id| url.contains(&format!("/{id}_")));
        // An animation's still is the first sunrise's.
        let png = &self.pngs[id.unwrap_or(1) - 1];
        to.write_all(png).expect("the test setup holds");
        Ok(png.len() as u64)
    }
}
