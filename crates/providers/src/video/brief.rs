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

use scorsese_core::{Asset, AssetId, AssetKind, Project, ProjectPath, VideoRequest, hash_bytes};

use super::GenerationError;
use crate::fingerprint::{self, Fingerprint};

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
            reference_images: read_stills(project, root, &request.reference_images)?,
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
        self.fingerprint().digest()
    }

    /// The text [`Brief::digest`] hashes.
    ///
    /// **Nothing may change it for a brief that already exists**: the digest
    /// names the file a paid-for shot is kept in, so a new line for every brief
    /// would make every generated shot look unpaid and quote it again. A field
    /// added to the request adds its line **only when it is set** — the test
    /// below pins a brief's digest to hold that. What Veo documents and this
    /// leaves out, and why, is beside [`Parameters`](crate::api::veo::request::Parameters).
    fn fingerprint(&self) -> Fingerprint {
        Fingerprint::of("veo")
            .line("model", self.request.model.as_str())
            .line("resolution", format!("{:?}", self.request.resolution))
            .line("seconds", self.request.seconds.get())
            .line("aspect", format!("{:?}", self.request.aspect))
            .line("prompt", &self.prompt)
            .or_none(
                "first",
                self.first_image.as_ref().map(|still| &still.digest),
            )
            .or_none("last", self.last_image.as_ref().map(|still| &still.digest))
            .each(
                "reference",
                self.reference_images.iter().map(|still| &still.digest),
            )
    }

    /// Where a generation of this brief lands, project-relative.
    ///
    /// Named for the asset **and** the fingerprint. The fingerprint alone would
    /// be enough for the cache and would even save money — two shots asking the
    /// same thing would share one file — but it is the wrong answer for an
    /// editor: two shots with the same prompt are two takes, and a cut that
    /// used one file twice would show the same eight seconds in both places.
    pub fn output(&self) -> ProjectPath {
        fingerprint::output(&self.id, &self.digest(), "mp4")
    }

    /// Whether a generation of this brief is already on disk.
    ///
    /// The answer to *has this been paid for*, whatever the document claims
    /// about the asset — and the **current** brief's file, never the asset's
    /// recorded `path`, which after an edit still points at the previous
    /// generation. Every decision about whether a run has work goes through
    /// here so it cannot be made both ways.
    pub fn realized(&self, root: &Path) -> bool {
        self.output().resolve(root).is_file()
    }
}

/// Reads every still `ids` names, in order — the references of a shot or of a
/// drawing, refused at the first one that cannot be read.
pub(crate) fn read_stills<'a>(
    project: &Project,
    root: &Path,
    ids: impl IntoIterator<Item = &'a AssetId>,
) -> Result<Vec<Still>, GenerationError> {
    ids.into_iter()
        .map(|id| read_still(project, root, id))
        .collect()
}

/// Reads the image asset `id` names — an imported still, or a generated one
/// that has been generated.
///
/// Shared with [`crate::image`], whose references are read the same way.
pub(crate) fn read_still(
    project: &Project,
    root: &Path,
    id: &AssetId,
) -> Result<Still, GenerationError> {
    let asset = project
        .asset(id)
        .ok_or_else(|| GenerationError::NoSuchStill { id: id.clone() })?;
    // A generated still that is a sketch, or stale, is not the picture its
    // brief describes yet — and a stale one still has the old file on disk,
    // which is exactly the picture that must not be sent.
    if asset.kind.is_generated() && (!asset.has_renderable_media() || asset.needs_generation()) {
        return Err(GenerationError::StillNotGenerated { id: id.clone() });
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A brief as one written before #1110, pinned by its digest. If this
    /// moves, every shot already generated would be quoted and paid for again.
    #[test]
    fn an_existing_briefs_fingerprint_does_not_move() {
        let brief = Brief {
            id: AssetId::new("shot"),
            prompt: String::from("a butterfly lands on a leaf"),
            request: VideoRequest::default(),
            first_image: None,
            last_image: None,
            reference_images: Vec::new(),
        };
        assert_eq!(
            brief.digest(),
            "c2ab690ac91c7657bc875836f4a9919be580102065626f40c6bb98fe597a9b7a"
        );
        assert_eq!(
            brief.output().as_str(),
            "generated/shot-c2ab690ac91c7657bc875836f4a9919be580102065626f40c6bb98fe597a9b7a.mp4"
        );
    }

    /// A brief with every still it can name, pinned the same way: the
    /// `first:`, `last:` and `reference:` lines with something in them.
    #[test]
    fn a_brief_with_stills_keeps_its_fingerprint() {
        let still = |id: &str, digest: &str| Still {
            id: AssetId::new(id),
            mime_type: String::from("image/png"),
            bytes: Vec::new(),
            digest: digest.to_owned(),
        };
        let brief = Brief {
            id: AssetId::new("shot"),
            prompt: String::from("a butterfly lands on a leaf"),
            request: VideoRequest::default(),
            first_image: Some(still("open", "aaa")),
            last_image: None,
            reference_images: vec![still("face", "bbb"), still("hat", "ccc")],
        };
        assert_eq!(
            brief.digest(),
            "cf6f0698c009166f90843bb6f6c7c7d651269374e74648291cb9c67e5c466702"
        );
    }
}
