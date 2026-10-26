//! The ffmpeg processes a render talks to.
//!
//! This is the whole of ffmpeg's job in scorsese: hand us raw frames and raw
//! samples from a source, and take them back to encode. Everything between —
//! what is on screen, where, how opaque, and how loud — happens in our process.
//! That is Path B, and these files are its edges.

mod audio;
mod decode;
mod encode;
mod listed;
mod partial;
mod process;
mod resized;

pub(crate) use audio::{ALIGN_TO_FILE_CLOCK, AudioDecoder, AudioSource, SAMPLE_FORMAT};
pub(crate) use decode::{Decoder, Fitting, Source, reads_through_image2};
pub(crate) use encode::{Encoder, encode_mix};
pub(crate) use listed::frames as listed_frames;
pub(crate) use partial::Partial;
pub(crate) use process::{Process, finish, finish_unless};
pub(crate) use resized::{Fitter, Pending};
