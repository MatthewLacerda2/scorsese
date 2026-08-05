//! What was asked for, resolved — and the fingerprint that keeps it from being
//! asked for twice.
//!
//! An asset's brief is spread across the document: a sentence in `prompt`, the
//! rest of the request in `video`, and stills named by **asset id** rather than
//! by path, because ids are what survives a file being re-imported. A [`Brief`]
//! is all of that gathered, with each still's bytes actually read — which is
//! also the point at which a brief can be wrong, so this is where it is
//! refused.
//!
//! # The money guarantee
//!
//! [`Brief::digest`] is a hash over **every field of the request and the bytes
//! of every still**, and the file a generation lands in is named for it. So an
//! unchanged brief is never billed twice, and — the case a name-based hash gets
//! wrong — swapping `face.png` for a different picture of the same name does
//! change the fingerprint, because it is the image's own `sha256` that goes in
//! rather than its file name.
//!
//! Nothing about *when* is in it. A timestamp inside a brief hash would make an
//! unchanged prompt look edited and pay for it again.

use std::path::Path;

use scorsese_core::{
    Asset, AssetId, AssetKind, GENERATED_DIR, Project, ProjectPath, VideoRequest, hash_bytes,
};

use super::GenerationError;

/// A still a brief names, read off disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Still {
    /// Which asset it came from.
    pub id: AssetId,
    /// What kind of picture it is, as a provider needs told: `image/png`.
    pub mime_type: String,
    /// The bytes.
    pub bytes: Vec<u8>,
    /// Their `sha256`, which is what reaches the fingerprint.
    pub digest: String,
}

/// Everything one generation asks for, gathered and resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct Brief {
    /// The asset this is the brief of.
    pub id: AssetId,
    /// The sentence.
    pub prompt: String,
    /// Model, resolution, length, aspect.
    pub request: VideoRequest,
    /// The frame to open on.
    pub first_image: Option<Still>,
    /// The frame to end on.
    pub last_image: Option<Still>,
    /// Stills of a subject that should keep looking like itself.
    pub reference_images: Vec<Still>,
}

impl Brief {
    /// Gathers the brief of `asset`, reading every still it names.
    ///
    /// Validation has already refused the impossible combinations — lite with
    /// reference images, a last frame with no first, more stills than the model
    /// takes. What is left for here is what only the disk can answer: whether
    /// the pictures are actually there.
    pub fn of(project: &Project, root: &Path, asset: &Asset) -> Result<Self, GenerationError> {
        if asset.kind != AssetKind::GeneratedVideo {
            return Err(GenerationError::NotAGeneratedVideo {
                id: asset.id.clone(),
                kind: asset.kind,
            });
        }
        let prompt = asset
            .prompt
            .clone()
            .filter(|prompt| !prompt.trim().is_empty())
            .ok_or_else(|| GenerationError::NoPrompt {
                id: asset.id.clone(),
            })?;
        let request = asset.video_request();
        let still = |id: &Option<AssetId>| -> Result<Option<Still>, GenerationError> {
            id.as_ref()
                .map(|id| read_still(project, root, id))
                .transpose()
        };

        Ok(Self {
            id: asset.id.clone(),
            prompt,
            first_image: still(&request.first_image)?,
            last_image: still(&request.last_image)?,
            reference_images: request
                .reference_images
                .iter()
                .map(|id| read_still(project, root, id))
                .collect::<Result<Vec<_>, _>>()?,
            request,
        })
    }

    /// The fingerprint of everything this asks for.
    ///
    /// Written out as lines rather than hashed field by field so the hashed
    /// text is something a person can print and read when a cache behaves in a
    /// way nobody expects. Every field is labelled, so two different briefs
    /// cannot collide by one field's value being another field's — a prompt
    /// ending in a number and an empty prompt followed by that number.
    pub fn digest(&self) -> String {
        hash_bytes(self.fingerprint().as_bytes())
    }

    /// The text [`Brief::digest`] hashes.
    fn fingerprint(&self) -> String {
        let mut text = String::from("veo\n");
        text.push_str(&format!("model:{}\n", self.request.model.as_str()));
        text.push_str(&format!("resolution:{:?}\n", self.request.resolution));
        text.push_str(&format!("seconds:{}\n", self.request.seconds.get()));
        text.push_str(&format!("aspect:{:?}\n", self.request.aspect));
        text.push_str(&format!("prompt:{}\n", self.prompt));
        text.push_str(&format!("first:{}\n", digest_of(self.first_image.as_ref())));
        text.push_str(&format!("last:{}\n", digest_of(self.last_image.as_ref())));
        for reference in &self.reference_images {
            text.push_str(&format!("reference:{}\n", reference.digest));
        }
        text
    }

    /// Where a generation of this brief lands, project-relative.
    ///
    /// Named for the asset **and** the fingerprint. The fingerprint alone would
    /// be enough for the cache and would even save money — two shots asking the
    /// same thing would share one file — but it is the wrong answer for an
    /// editor: two shots with the same prompt are two takes, and a cut that
    /// used one file twice would show the same eight seconds in both places.
    pub fn output(&self) -> ProjectPath {
        ProjectPath::new(format!("{GENERATED_DIR}/{}-{}.mp4", self.id, self.digest()))
    }
}

/// A still's digest, or the word that stands in for one that is not there.
///
/// `none` rather than an empty string, so a brief with no first frame and a
/// brief whose first frame hashed to nothing are different documents.
fn digest_of(still: Option<&Still>) -> &str {
    still.map_or("none", |still| still.digest.as_str())
}

/// Reads the image asset `id` names.
fn read_still(project: &Project, root: &Path, id: &AssetId) -> Result<Still, GenerationError> {
    let asset = project
        .asset(id)
        .ok_or_else(|| GenerationError::NoSuchStill { id: id.clone() })?;
    let path = asset
        .path
        .as_ref()
        .ok_or_else(|| GenerationError::StillHasNoFile { id: id.clone() })?;
    let file = path.resolve(root);
    let bytes = std::fs::read(&file).map_err(|source| GenerationError::ReadStill {
        id: id.clone(),
        path: file,
        source,
    })?;
    Ok(Still {
        digest: hash_bytes(&bytes),
        mime_type: mime_of(path).to_owned(),
        bytes,
        id: id.clone(),
    })
}

/// What kind of picture a path holds, by its extension.
///
/// PNG and JPEG are what Veo takes, and an extension it does not know is
/// refused here rather than by the provider — a local refusal costs nothing and
/// names the asset, and a remote one costs a round trip to say less.
fn mime_of(path: &ProjectPath) -> &'static str {
    let lower = path.as_str().to_ascii_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else {
        "image/png"
    }
}
