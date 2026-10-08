//! A stock library that answers from memory and remembers what it was asked
//! to download.

use std::io::Write;
use std::sync::{Arc, Mutex};

use scorsese_providers::stock::{Candidate, Library, Medium, Page, Query, Rendition, StockError};

use super::common;

/// Images 1 to 3, each a picture of a sunrise in a colour of its own; it
/// remembers what it was asked to download.
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

impl Library for Fake {
    fn name(&self) -> &'static str {
        "Fake"
    }

    fn page(&self, _: &Query, _: u32) -> Result<Page, StockError> {
        let candidates = (1..=3).map(picture).collect();
        Ok(Page {
            candidates,
            total: 3,
        })
    }

    fn one(&self, _: Medium, id: u64) -> Result<Option<Candidate>, StockError> {
        Ok((1..=3).contains(&id).then(|| picture(id)))
    }

    fn download(&self, url: &str, _: u64, to: &mut dyn Write) -> Result<u64, StockError> {
        let mut downloaded = self.downloaded.lock().expect("the test setup holds");
        downloaded.push(url.to_owned());
        let id = (1..=3).find(|id| url.contains(&format!("/{id}_")));
        let png = &self.pngs[id.expect("one of the fake's") - 1];
        to.write_all(png).expect("the test setup holds");
        Ok(png.len() as u64)
    }
}
