//! A library that answers from memory and counts what it was asked.

use std::cell::Cell;
use std::io::Write;
use std::path::Path;

use scorsese_core::{Fps, MediaMetadata, ProbeError, ProbeMedia};
use scorsese_providers::stock::{Candidate, Library, Medium, Page, Query, Rendition, StockError};

/// `count` results for any query, `vertical_every` of them taller than wide.
pub(crate) struct Fake {
    pub(crate) count: u64,
    pub(crate) vertical_every: u64,
    pub(crate) pages: Cell<u32>,
    pub(crate) ones: Cell<u32>,
    pub(crate) downloads: Cell<u32>,
}

impl Fake {
    pub(crate) fn with(count: u64) -> Self {
        Self {
            count,
            vertical_every: 0,
            pages: Cell::new(0),
            ones: Cell::new(0),
            downloads: Cell::new(0),
        }
    }
}

/// Result `id`: horizontal unless `vertical`, with renditions up to 4K.
pub(crate) fn candidate(medium: Medium, id: u64, vertical: bool) -> Candidate {
    let rendition = |name: &str, width: u32, height: u32| {
        let (width, height) = if vertical {
            (height, width)
        } else {
            (width, height)
        };
        Rendition {
            name: name.to_owned(),
            url: format!("https://cdn.example.invalid/{id}_{name}.mp4"),
            width,
            height,
            bytes: None,
        }
    };
    if medium == Medium::Lottie {
        return animation(id);
    }
    Candidate {
        medium,
        id,
        title: String::new(),
        style: String::from("film"),
        tags: vec![String::from("sunrise")],
        seconds: Some(10),
        fps: None,
        author: String::from("someone"),
        page_url: format!("https://pixabay.com/videos/id-{id}/"),
        preview_url: format!("https://cdn.example.invalid/{id}_tiny.jpg"),
        motion_url: None,
        ai_generated: false,
        renditions: vec![
            rendition("tiny", 640, 360),
            rendition("medium", 1920, 1080),
            rendition("large", 3840, 2160),
        ],
    }
}

/// Animation `id`: its JSON the one rendition, an MP4 of it to look through.
/// Id 13 downloads as something that is not a Lottie.
fn animation(id: u64) -> Candidate {
    Candidate {
        medium: Medium::Lottie,
        id,
        title: format!("Wave {id}"),
        style: String::new(),
        tags: Vec::new(),
        seconds: Some(2),
        fps: Some(30),
        author: String::from("someone"),
        page_url: format!("https://lottiefiles.com/animations/wave-{id}"),
        preview_url: format!("https://cdn.example.invalid/{id}.png"),
        motion_url: Some(format!("https://cdn.example.invalid/{id}.mp4")),
        ai_generated: false,
        renditions: vec![Rendition {
            name: String::from("json"),
            url: format!("https://cdn.example.invalid/{id}.json"),
            width: 512,
            height: 512,
            bytes: None,
        }],
    }
}

/// A Lottie file as small as one can be, the bytes the fake downloads for
/// an animation's JSON.
pub(crate) const LOTTIE: &str =
    r#"{"v":"5.7.0","fr":30,"ip":0,"op":60,"w":512,"h":512,"layers":[],"assets":[]}"#;

impl Library for Fake {
    fn name(&self) -> &'static str {
        "Fake"
    }

    fn page(&self, query: &Query, page: u32) -> Result<Page, StockError> {
        self.pages.set(self.pages.get() + 1);
        let size = u64::from(scorsese_providers::stock::PAGE_SIZE);
        let start = u64::from(page - 1) * size;
        let candidates = (start..self.count.min(start + size))
            .map(|id| {
                let vertical = self.vertical_every > 0 && id % self.vertical_every == 0;
                candidate(query.medium, id + 1, vertical)
            })
            .collect();
        Ok(Page {
            candidates,
            total: self.count,
        })
    }

    fn one(&self, medium: Medium, id: u64) -> Result<Option<Candidate>, StockError> {
        self.ones.set(self.ones.get() + 1);
        Ok((id <= self.count).then(|| candidate(medium, id, false)))
    }

    fn download(&self, url: &str, _: u64, to: &mut dyn Write) -> Result<u64, StockError> {
        self.downloads.set(self.downloads.get() + 1);
        let bytes = match url {
            "https://cdn.example.invalid/13.json" => String::from("<html>not found</html>"),
            _ if url.ends_with(".json") => LOTTIE.to_owned(),
            _ => format!("bytes of {url}"),
        };
        to.write_all(bytes.as_bytes())
            .map_err(|source| StockError::Io {
                path: url.into(),
                source,
            })?;
        Ok(bytes.len() as u64)
    }
}

/// A prober that calls every file a 10-second 1080p video.
pub(crate) struct Probe;

impl ProbeMedia for Probe {
    fn probe(&self, _: &Path) -> Result<MediaMetadata, ProbeError> {
        Ok(MediaMetadata {
            duration_seconds: Some(10.0),
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(Fps::THIRTY),
            has_alpha: Some(false),
            ..MediaMetadata::default()
        })
    }
}

/// A query for footage of `words`, unfiltered.
pub(crate) fn query(words: &str) -> Query {
    Query {
        medium: Medium::Video,
        words: words.to_owned(),
        style: None,
        orientation: None,
        min_seconds: None,
        safe: true,
    }
}
