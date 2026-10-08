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

use scorsese_core::{
    Asset, AssetId, AssetKind, GENERATED_DIR, ImageRequest, Project, ProjectPath, hash_bytes,
};

use super::Incomplete;
use crate::video::Still;
use crate::video::brief::read_still;

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
        let reference_images = request
            .references()
            .map(|id| read_still(project, root, id))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| Incomplete::Reference {
                id: asset.id.clone(),
                why: error.to_string(),
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
        hash_bytes(self.fingerprint().as_bytes())
    }

    /// The text [`Brief::digest`] hashes: a labelled line per field, so one
    /// field's value cannot run into the next.
    fn fingerprint(&self) -> String {
        let mut text = String::from("gemini-image\n");
        text.push_str(&format!("model:{}\n", self.request.model.as_str()));
        text.push_str(&format!("resolution:{}\n", self.request.size().as_str()));
        text.push_str(&format!("aspect:{}\n", self.request.aspect.as_str()));
        if self.request.thinking() != self.request.model.default_thinking()
            && let Some(level) = self.request.thinking()
        {
            text.push_str(&format!("thinking:{}\n", level.as_str()));
        }
        text.push_str(&format!("prompt:{}\n", self.prompt));
        for reference in &self.reference_images {
            text.push_str(&format!("reference:{}\n", reference.digest));
        }
        text
    }

    /// Where a drawing of this brief lands, project-relative — named for the
    /// asset and the fingerprint, for [`video`](crate::video::Brief::output)'s
    /// reason: two stills with the same prompt are two takes.
    pub fn output(&self) -> ProjectPath {
        ProjectPath::new(format!("{GENERATED_DIR}/{}-{}.jpg", self.id, self.digest()))
    }

    /// Whether a drawing of this brief is already on disk — the answer to *has
    /// this been paid for*, from the **current** brief's file.
    pub fn realized(&self, root: &Path) -> bool {
        self.output().resolve(root).is_file()
    }
}
