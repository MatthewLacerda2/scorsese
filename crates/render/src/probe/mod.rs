//! Reading technical metadata out of media files.

mod ffprobe;
mod fill;
mod report;

pub use ffprobe::Ffprobe;
pub use fill::fill_media;
