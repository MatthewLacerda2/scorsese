//! What was asked for, resolved — and the fingerprint that keeps it from being
//! asked for twice.
//!
//! [`video::brief`](crate::video::Brief)'s shape, smaller: a sentence, a
//! model, a size, an aspect, and the reference pictures read off disk.
//!
//! # The money guarantee
//!
//! [`Brief::digest`] hashes every field of the request **and the bytes of
//! every reference**, and the file a drawing lands in is named for it — so an
//! unchanged brief is never billed twice, and a reference regenerated under
//! the same id is a new brief. The **size** goes in resolved, the model's
//! default included, so writing the default out explicitly changes nothing; a
//! thinking level goes in only when it is not the model's default, for the same
//! reason, and so a brief written before levels existed keeps its fingerprint.
//! A reference's **kind** is not in it: the vendor is never told it, so moving
//! a picture from objects to characters sends the same request.
//! The asset's `note` is not in it: a note is handed to nobody.

use std::path::Path;

use scorsese_core::{Asset, AssetId, AssetKind, ImageRequest, Project, ProjectPath};

use super::Incomplete;
use crate::fingerprint::{self, Fingerprint};
use crate::video::Still;
use crate::video::brief::read_stills;

/// Everything one still asks for, gathered and resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct Brief {
    /// The asset this is the brief of.
    pub id: AssetId,
    /// The sentence.
    pub prompt: String,
    /// Model, size, aspect, thinking, and which reference is which kind.
    pub request: ImageRequest,
    /// Every reference picture, read off disk in the order they are sent:
    /// objects, then characters, then styles.
    pub reference_images: Vec<Still>,
}

impl Brief {
    /// Gathers the brief of `asset`, reading every reference it names, or says
    /// what it is still missing.
    pub fn of(project: &Project, root: &Path, asset: &Asset) -> Result<Self, Incomplete> {
        if asset.kind != AssetKind::GeneratedImage {
            return Err(Incomplete::NotGeneratedImage {
                id: asset.id.clone(),
                kind: asset.kind,
            });
        }
        let prompt = asset
            .prompt
            .clone()
            .filter(|prompt| !prompt.trim().is_empty())
            .ok_or_else(|| Incomplete::NoPrompt {
                id: asset.id.clone(),
            })?;
        let request = asset.image_request();
        let reference_images =
            read_stills(project, root, request.references()).map_err(|error| {
                Incomplete::Reference {
                    id: asset.id.clone(),
                    why: error.to_string(),
                }
            })?;
        Ok(Self {
            id: asset.id.clone(),
            prompt,
            request,
            reference_images,
        })
    }

    /// How many characters of prompt are sent — what the input estimate
    /// approximates tokens from.
    pub fn characters(&self) -> usize {
        self.prompt.chars().count()
    }

    /// The fingerprint of everything this asks for.
    pub fn digest(&self) -> String {
        self.fingerprint().digest()
    }

    /// The text [`Brief::digest`] hashes: a labelled line per field, so one
    /// field's value cannot run into the next.
    fn fingerprint(&self) -> Fingerprint {
        let thinking = self.request.thinking();
        let thinking = thinking.filter(|_| thinking != self.request.model.default_thinking());
        Fingerprint::of("gemini-image")
            .line("model", self.request.model.as_str())
            .line("resolution", self.request.size().as_str())
            .line("aspect", self.request.aspect.as_str())
            .when("thinking", thinking.map(|level| level.as_str()))
            .line("prompt", &self.prompt)
            .each(
                "reference",
                self.reference_images.iter().map(|still| &still.digest),
            )
    }

    /// Where a drawing of this brief lands, project-relative — named for the
    /// asset and the fingerprint, for [`video`](crate::video::Brief::output)'s
    /// reason: two stills with the same prompt are two takes.
    pub fn output(&self) -> ProjectPath {
        fingerprint::output(&self.id, &self.digest(), "jpg")
    }

    /// The name of [`Brief::output`]'s file without its directory or
    /// extension — what a batch request is keyed by, so its answer finds the
    /// file it belongs in.
    pub fn key(&self) -> String {
        fingerprint::key(&self.id, &self.digest())
    }

    /// Whether a drawing of this brief is already on disk — the answer to *has
    /// this been paid for*, from the **current** brief's file.
    pub fn realized(&self, root: &Path) -> bool {
        self.output().resolve(root).is_file()
    }
}

#[cfg(test)]
mod tests {
    use scorsese_core::{ImageModel, ImageThinking};

    use super::*;

    /// Stills already drawn, pinned by their digests: one as plain as a brief
    /// gets, one with a thinking level off its model's default and a
    /// reference. If either moves, every still already paid for would be
    /// quoted and paid for again.
    #[test]
    fn an_existing_briefs_fingerprint_does_not_move() {
        let mut brief = Brief {
            id: AssetId::new("still"),
            prompt: String::from("a lighthouse at dusk"),
            request: ImageRequest::default(),
            reference_images: Vec::new(),
        };
        assert_eq!(
            brief.digest(),
            "9e02772b372afada344fc7db5b435e26439adaf6b455d19438cdb43d51ee3acc"
        );
        assert_eq!(
            brief.output().as_str(),
            "generated/still-9e02772b372afada344fc7db5b435e26439adaf6b455d19438cdb43d51ee3acc.jpg"
        );
        brief.request.model = ImageModel::Flash;
        brief.request.thinking = Some(ImageThinking::High);
        brief.reference_images.push(Still {
            id: AssetId::new("face"),
            mime_type: String::from("image/png"),
            bytes: Vec::new(),
            digest: String::from("abc"),
        });
        assert_eq!(
            brief.digest(),
            "3d33eed4bc83894ac81d9661c249d8aab7627a5b5b4fdc31f25e0c33254aa0c6"
        );
    }
}
