//! Image-sequence checks: what the named stills turn out to be.
//!
//! Whether the block is there at all is [`super::assets`]'s question, asked of
//! every kind's own block in one place; this is what is inside it.

use crate::asset::{Asset, AssetKind, SEQUENCE_FORMATS};
use crate::project::Project;

use super::error::{AssetProblem, SequenceProblem};

pub(super) fn check(project: &Project, asset: &Asset, errors: &mut Vec<AssetProblem>) {
    let Some(sequence) = &asset.sequence else {
        return;
    };
    if asset.kind != AssetKind::ImageSequence {
        // Reported as a stray field by the assets pass.
        return;
    }
    let id = || asset.id.clone();
    let mut found = Vec::new();
    if sequence.stills.is_empty() {
        found.push(SequenceProblem::NoStills { asset: id() });
    }
    if sequence.hold.get() == 0 {
        found.push(SequenceProblem::NoHold { asset: id() });
    }

    // The first still that is a picture sets the format and size every later
    // one is held to; each mismatch is reported once, at its first instance.
    let mut format: Option<String> = None;
    let mut size: Option<(u32, u32)> = None;
    let (mut mixed_format, mut mixed_size) = (false, false);
    for still_id in &sequence.stills {
        let still = match project.asset(still_id) {
            None => {
                found.push(SequenceProblem::UnknownStill {
                    asset: id(),
                    still: still_id.clone(),
                });
                continue;
            }
            Some(still) if still.kind != AssetKind::Image => {
                found.push(SequenceProblem::NotAnImage {
                    asset: id(),
                    still: still_id.clone(),
                    kind: still.kind,
                });
                continue;
            }
            Some(still) => still,
        };
        let this = format_of(still);
        if !SEQUENCE_FORMATS.contains(&this.as_str()) {
            found.push(SequenceProblem::NotOnePicture {
                asset: id(),
                still: still_id.clone(),
                format: this,
            });
        } else {
            match &format {
                None => format = Some(this),
                Some(expected)
                    if same_decoder(expected) != same_decoder(&this) && !mixed_format =>
                {
                    mixed_format = true;
                    found.push(SequenceProblem::MixedFormats {
                        asset: id(),
                        still: still_id.clone(),
                        expected: expected.clone(),
                        found: this,
                    });
                }
                Some(_) => {}
            }
        }
        let measured = still.media.and_then(|m| Some((m.width?, m.height?)));
        if let Some((found_width, found_height)) = measured {
            match size {
                None => size = Some((found_width, found_height)),
                Some((width, height))
                    if (width, height) != (found_width, found_height) && !mixed_size =>
                {
                    mixed_size = true;
                    found.push(SequenceProblem::MixedSizes {
                        asset: id(),
                        still: still_id.clone(),
                        width,
                        height,
                        found_width,
                        found_height,
                    });
                }
                Some(_) => {}
            }
        }
    }
    errors.extend(found.into_iter().map(Into::into));
}

/// A still's file format, read off its extension, lowercased — empty when it
/// has none.
fn format_of(still: &Asset) -> String {
    still
        .path
        .as_ref()
        .and_then(|path| {
            std::path::Path::new(path.as_str())
                .extension()?
                .to_str()
                .map(str::to_ascii_lowercase)
        })
        .unwrap_or_default()
}

/// One decoder reads both spellings of these, so they are one format.
fn same_decoder(format: &str) -> &str {
    match format {
        "jpeg" => "jpg",
        "tiff" => "tif",
        other => other,
    }
}
