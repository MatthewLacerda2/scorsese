//! The text a brief's digest hashes, and the file a generation of it lands in
//! — one set of rules for a shot, a still, a line, a voice design and a quote
//! (#1279).
//!
//! Every brief here is hashed the same way: a labelled line per field, so the
//! hashed text is something a person can print and read when a cache behaves
//! in a way nobody expects, and so one field's value can never run into the
//! next — a prompt ending in a number and an empty prompt followed by that
//! number. Each brief used to build that text by hand, with its own spelling
//! of the `none` that stands in for an absent field; [`Fingerprint`] is that
//! text built once.
//!
//! # The text is a compatibility surface
//!
//! A digest names the file a paid-for generation is kept in, and a quote's
//! digest is what a token is bound to. So **nothing here may change the bytes
//! a brief already hashes to**: a different byte re-bills every generation in
//! every project. Each brief pins its digest in a test, and those tests are
//! what this builder answers to.

use std::fmt::Display;

use scorsese_core::{AssetId, GENERATED_DIR, ProjectPath, hash_bytes};

/// A brief's hashed text, a labelled line at a time.
///
/// Ordered by its text, which is how a quote sorts its charges.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Fingerprint(String);

impl Fingerprint {
    /// A fingerprint opening on the vendor's word, which keeps two vendors'
    /// briefs apart even if every field agreed.
    pub(crate) fn of(vendor: &str) -> Self {
        Self(format!("{vendor}\n"))
    }

    /// `label:value`, always written.
    pub(crate) fn line(mut self, label: &str, value: impl Display) -> Self {
        self.0.push_str(&format!("{label}:{value}\n"));
        self
    }

    /// An optional field, written `none` when it is absent.
    ///
    /// `none` rather than an empty string, so a brief without the field and a
    /// brief whose field was blank are different documents.
    pub(crate) fn or_none(self, label: &str, value: Option<impl Display>) -> Self {
        match value {
            Some(value) => self.line(label, value),
            None => self.line(label, "none"),
        }
    }

    /// A field written only when it is set — the shape of a field added after
    /// briefs already existed, so the briefs that never set it keep their
    /// digest.
    pub(crate) fn when(self, label: &str, value: Option<impl Display>) -> Self {
        match value {
            Some(value) => self.line(label, value),
            None => self,
        }
    }

    /// One `label:value` line per value, in order.
    pub(crate) fn each(self, label: &str, values: impl IntoIterator<Item = impl Display>) -> Self {
        values
            .into_iter()
            .fold(self, |text, value| text.line(label, value))
    }

    /// This text with `more` after it.
    pub(crate) fn then(mut self, more: Self) -> Self {
        self.0.push_str(&more.0);
        self
    }

    /// The `sha256` of the text, which is what names a generation's file.
    pub(crate) fn digest(&self) -> String {
        hash_bytes(self.0.as_bytes())
    }
}

/// The name a generation of an asset's brief goes by: the asset **and** the
/// digest.
///
/// The digest alone would be enough for the cache and would even save money —
/// two assets asking the same thing would share one file — but it is the wrong
/// answer for an editor: two shots with the same prompt are two takes, and a
/// cut that used one file twice would show the same eight seconds in both
/// places.
pub(crate) fn key(id: &AssetId, digest: &str) -> String {
    format!("{id}-{digest}")
}

/// Where a generation of an asset's brief lands, project-relative: its
/// [`key`] in `generated/`, with the extension the vendor's file has.
pub(crate) fn output(id: &AssetId, digest: &str, extension: &str) -> ProjectPath {
    ProjectPath::new(format!("{GENERATED_DIR}/{}.{extension}", key(id, digest)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fingerprint_is_labelled_lines_with_none_for_what_is_absent() {
        let text = Fingerprint::of("vendor")
            .line("model", "fast")
            .or_none("seed", None::<u32>)
            .or_none("language", Some("pt"))
            .when("thinking", None::<&str>)
            .when("level", Some(2))
            .each("reference", ["a", "b"])
            .then(Fingerprint::default().line("tail", 1));
        assert_eq!(
            text.0,
            "vendor\nmodel:fast\nseed:none\nlanguage:pt\nlevel:2\nreference:a\nreference:b\ntail:1\n"
        );
        assert_eq!(text.digest(), hash_bytes(text.0.as_bytes()));
    }

    #[test]
    fn an_output_is_named_for_the_asset_and_the_digest() {
        let id = AssetId::new("shot");
        assert_eq!(key(&id, "abc"), "shot-abc");
        assert_eq!(output(&id, "abc", "mp4").as_str(), "generated/shot-abc.mp4");
    }
}
