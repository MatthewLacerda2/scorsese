//! Choosing a voice, driven end to end against a scripted catalogue.
//!
//! **Nothing here reaches a network or needs a key**, and that is the point of
//! `Catalogue` being a trait rather than a struct. It is also what makes the
//! interesting states reachable at all: a voice that has been withdrawn, a
//! Voice Library refused to a free plan, and a key that is valid but never
//! granted `voices_read` are each one line of a fake here, and each would
//! otherwise need somebody's billing details and a wait until 2027.

#[path = "../common/mod.rs"]
mod common;

mod caching;
mod fake;
mod recovery;

use scorsese_providers::voices::Voice;

use common::project;

/// A voice with the fields a listing actually fills in.
fn voice(id: &str, name: &str) -> Voice {
    Voice {
        id: String::from(id),
        name: String::from(name),
        traits: vec![String::from("pt"), String::from("female")],
        description: Some(String::from("warm, unhurried")),
        preview: None,
    }
}

/// A project directory to cache into, and nothing else in it.
fn root(label: &str) -> std::path::PathBuf {
    project(label).0
}
