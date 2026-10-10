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
//!
//! **A song that names a patch file depends on that file too** (#672). Its
//! bytes are as much a part of what the render reads as the song's own, so
//! each named patch joins the fingerprint as one more labelled line, in track
//! order. A one-shot or an all-inline song names none and gains no line, which
//! keeps every address already on disk exactly where it was.
//!
//! **So does what a song read from the project** (#1000): a `fit` to the clip
//! that plays it renders at that clip's length, so the length joins as one more
//! labelled line (`placed`), and a song that read nothing gains none.

use scorsese_core::{GENERATED_DIR, ProjectPath, hash_bytes};
use scorsese_zimmer::SYNTH_VERSION;

/// One patch a song names, as the address sees it: the reference as written,
/// and the sha256 of the file it resolves to.
///
/// The digest is `None` when the file cannot be read. That still moves the
/// address — and to one nothing is ever written at, because the render that
/// follows fails on the same missing file and says why.
pub(crate) type Named = (String, Option<String>);

/// Where the bake of the recipe hashing to `recipe` lands, project-relative.
///
/// `recipe` is the sha256 of the recipe file's own bytes, and `patches` the
/// files a song names (empty for anything else). What comes back is
/// the address of the file **this build's synthesiser** would write there, so
/// a bake left by an older one is simply not at it — a miss, and the ordinary
/// re-render that a miss already means.
///
/// `placed` is what the song read from the project — one already-labelled
/// line per fact, from `placed::resolve`, and empty for a recipe that asks it
/// nothing.
pub(crate) fn output(recipe: &str, patches: &[Named], placed: &[String]) -> ProjectPath {
    let digest = digest(recipe, SYNTH_VERSION, patches, placed);
    ProjectPath::new(format!("{GENERATED_DIR}/{digest}.wav"))
}

/// The digest that names a bake: the fingerprint, hashed.
///
/// Takes the version rather than reading the constant so the thing this module
/// exists to guarantee — that a different synthesiser is a different address —
/// is something a test can state directly instead of inferring from a number
/// nobody can change at runtime.
fn digest(recipe: &str, version: u32, patches: &[Named], placed: &[String]) -> String {
    hash_bytes(fingerprint(recipe, version, patches, placed).as_bytes())
}

/// The text [`digest`] hashes.
fn fingerprint(recipe: &str, version: u32, patches: &[Named], placed: &[String]) -> String {
    let mut text = format!("zimmer\nsynth:{version}\nrecipe:{recipe}\n");
    for (reference, sha256) in patches {
        let sha256 = sha256.as_deref().unwrap_or("unreadable");
        text.push_str(&format!("patch:{reference}:{sha256}\n"));
    }
    for fact in placed {
        text.push_str(&format!("placed:{fact}\n"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of the module: the same document, a different
    /// synthesiser, a different file.
    #[test]
    fn a_new_synthesiser_moves_the_address() {
        assert_ne!(digest("abc", 1, &[], &[]), digest("abc", 2, &[], &[]));
    }

    #[test]
    fn a_new_recipe_still_moves_the_address() {
        assert_ne!(digest("abc", 1, &[], &[]), digest("abd", 1, &[], &[]));
    }

    /// Labelled lines, so a recipe digest cannot be read as part of a version
    /// number and the text stays something a person can print.
    #[test]
    fn the_fingerprint_names_both_of_its_parts() {
        let text = fingerprint("abc", 7, &[], &[]);
        assert!(text.contains("synth:7\n"), "got {text:?}");
        assert!(text.contains("recipe:abc\n"), "got {text:?}");
    }

    /// A bake is not addressed by the recipe's hash alone any more. If this
    /// fails, a file left by an older synthesiser is being served as fresh.
    #[test]
    fn the_bake_is_not_named_for_the_recipe_alone() {
        let path = output("abc", &[], &[]);
        assert!(path.as_str().starts_with("generated/"), "got {path}");
        assert!(path.as_str().ends_with(".wav"), "got {path}");
        assert!(!path.as_str().contains("abc"), "got {path}");
    }

    /// A song naming no patch file keeps the address it had before #672,
    /// byte for byte — pinned as a literal, so a change to the fingerprint's
    /// shape that would orphan every bake on disk cannot pass unnoticed.
    #[test]
    fn a_recipe_naming_no_patch_keeps_its_address() {
        assert_eq!(
            digest("abc", 7, &[], &[]),
            "fce9b89d778e45970eaebe3418fc94891ef9353b61400bfce48f78a0a24f49eb"
        );
    }

    /// The bug itself: the same song, a different patch file behind one of
    /// its names, a different address.
    #[test]
    fn an_edited_patch_file_moves_the_address() {
        let named = |sha: &str| vec![("recipes/bass.json".to_owned(), Some(sha.to_owned()))];
        assert_ne!(
            digest("abc", 7, &named("one"), &[]),
            digest("abc", 7, &named("two"), &[])
        );
        assert_ne!(
            digest("abc", 7, &named("one"), &[]),
            digest("abc", 7, &[], &[])
        );
    }

    /// The length a song was fitted to is part of its address: a clip made
    /// shorter is a re-bake, and a song that read nothing keeps its address.
    #[test]
    fn a_length_read_from_the_project_moves_the_address() {
        let at = |seconds: &str| output("abc", &[], &[format!("fit.to:clip={seconds}")]);
        assert_ne!(at("12"), at("14.5"));
        assert_ne!(at("12"), output("abc", &[], &[]));
        assert_eq!(at("12"), at("12"), "the same length is the same bake");
    }

    /// Each named patch is its own labelled line, and one that cannot be read
    /// is still named rather than left out.
    #[test]
    fn named_patches_are_labelled_lines() {
        let patches = vec![
            ("recipes/bass.json".to_owned(), Some("def".to_owned())),
            ("recipes/gone.json".to_owned(), None),
        ];
        let text = fingerprint("abc", 7, &patches, &[]);
        assert!(
            text.ends_with("patch:recipes/bass.json:def\npatch:recipes/gone.json:unreadable\n"),
            "got {text:?}"
        );
    }
}
