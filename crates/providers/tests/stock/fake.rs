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
    Candidate {
        medium,
        id,
        style: String::from("film"),
        tags: vec![String::from("sunrise")],
        seconds: Some(10),
        author: String::from("someone"),
        page_url: format!("https://pixabay.com/videos/id-{id}/"),
        preview_url: format!("https://cdn.example.invalid/{id}_tiny.jpg"),
        ai_generated: false,
        renditions: vec![
            rendition("tiny", 640, 360),
            rendition("medium", 1920, 1080),
            rendition("large", 3840, 2160),
        ],
    }
}

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
        let bytes = format!("bytes of {url}");
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
