//! Every integration test of `render`, in one binary (#1128).
//!
//! Each module is what used to be a test binary of its own; one binary
//! links once instead of once per file.

mod animation;
mod annotations;
mod arithmetic;
mod audio;
mod checkup;
mod common;
mod contact;
mod describing;
mod filling;
mod formats;
mod layouts;
mod pipeline;
mod plan;
mod previews;
mod probe;
mod references;
mod stills;
mod vocabulary;
mod warnings;
