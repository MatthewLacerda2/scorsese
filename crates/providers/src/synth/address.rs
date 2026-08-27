//! Where a bake lands, and what that address is made of.
//!
//! A bake is content-addressed, and the content is **both arguments to the
//! render**: the recipe document, and the synthesiser that reads it. Hashing
//! only the first is what let a DSP change leave every project holding audio
//! its own recipe no longer describes, under a key that still looked fresh.
//!
//! Written as a fingerprint text and hashed, rather than as fields folded into
//! a hash one at a time — the same shape `video` and `speech` briefs use, and
//! for the same two reasons: the hashed text is something a person can print
//! and read when a cache behaves in a way nobody expects, and labelled lines
//! cannot collide by one field's value running into the next.

use scorsese_core::{GENERATED_DIR, ProjectPath, hash_bytes};
use scorsese_zimmer::SYNTH_VERSION;

/// Where the bake of the recipe hashing to `recipe` lands, project-relative.
///
/// `recipe` is the sha256 of the recipe file's own bytes. What comes back is
/// the address of the file **this build's synthesiser** would write there, so
/// a bake left by an older one is simply not at it — a miss, and the ordinary
/// re-render that a miss already means.
pub(crate) fn output(recipe: &str) -> ProjectPath {
    ProjectPath::new(format!(
        "{GENERATED_DIR}/{}.wav",
        digest(recipe, SYNTH_VERSION)
    ))
}

/// The digest that names a bake: the fingerprint, hashed.
///
/// Takes the version rather than reading the constant so the thing this module
/// exists to guarantee — that a different synthesiser is a different address —
/// is something a test can state directly instead of inferring from a number
/// nobody can change at runtime.
fn digest(recipe: &str, version: u32) -> String {
    hash_bytes(fingerprint(recipe, version).as_bytes())
}

/// The text [`digest`] hashes.
fn fingerprint(recipe: &str, version: u32) -> String {
    format!("zimmer\nsynth:{version}\nrecipe:{recipe}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of the module: the same document, a different
    /// synthesiser, a different file.
    #[test]
    fn a_new_synthesiser_moves_the_address() {
        assert_ne!(digest("abc", 1), digest("abc", 2));
    }

    #[test]
    fn a_new_recipe_still_moves_the_address() {
        assert_ne!(digest("abc", 1), digest("abd", 1));
    }

    /// Labelled lines, so a recipe digest cannot be read as part of a version
    /// number and the text stays something a person can print.
    #[test]
    fn the_fingerprint_names_both_of_its_parts() {
        let text = fingerprint("abc", 7);
        assert!(text.contains("synth:7\n"), "got {text:?}");
        assert!(text.contains("recipe:abc\n"), "got {text:?}");
    }

    /// A bake is not addressed by the recipe's hash alone any more. If this
    /// fails, a file left by an older synthesiser is being served as fresh.
    #[test]
    fn the_bake_is_not_named_for_the_recipe_alone() {
        let path = output("abc");
        assert!(path.as_str().starts_with("generated/"), "got {path}");
        assert!(path.as_str().ends_with(".wav"), "got {path}");
        assert!(!path.as_str().contains("abc"), "got {path}");
    }
}
