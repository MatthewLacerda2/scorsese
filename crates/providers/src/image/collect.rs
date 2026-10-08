//! Collecting stills from their batches (#894): queued → generated.
//!
//! What every run of stills does first, and all a collecting run does. It
//! **never orders anything**, so it cannot spend: asking after a batch is
//! free. Each job is asked after once, however many stills wait in it.
//!
//! **Every picture a finished job carries is kept**, under the key it was
//! ordered with — the file its brief lands in — including one whose still has
//! been edited since. It was paid for; the day that brief comes back it is a
//! cache hit. The still itself only takes a picture drawn from its
//! **current** brief, and otherwise goes back to being a sketch with the
//! reason, as a still whose batch failed does.

use std::path::Path;

use scorsese_core::{AssetId, AssetKind, GENERATED_DIR, GenerationState, Project, ProjectPath};

use super::batch::half;
use super::run::{record, write};
use super::{Batch, Brief, ImageError, ImageProvider, Outcome};

/// Asks after every batch a still waits in, and does what each answer calls
/// for. Saving the project is the caller's.
pub fn collect(
    project: &mut Project,
    root: &Path,
    provider: &dyn ImageProvider,
) -> Result<Vec<(AssetId, Outcome)>, ImageError> {
    let mut done = Vec::new();
    for operation in operations(project) {
        let waiting = members(project, &operation);
        match provider.ask(&operation)? {
            Batch::Running => {
                for id in waiting {
                    let operation = operation.clone();
                    done.push((id, Outcome::Waiting { operation }));
                }
            }
            Batch::Stopped(why) => {
                for id in waiting {
                    release(project, &id);
                    done.push((
                        id,
                        Outcome::Failed {
                            message: why.clone(),
                        },
                    ));
                }
            }
            Batch::Finished(answers) => {
                keep(root, &answers)?;
                for id in waiting {
                    let outcome = land(project, root, &id, &answers)?;
                    done.push((id, outcome));
                }
            }
        }
    }
    Ok(done)
}

/// Writes every picture the job drew into `generated/`, under its key.
fn keep(root: &Path, answers: &[(String, Result<Vec<u8>, String>)]) -> Result<(), ImageError> {
    for (key, picture) in answers {
        let (Some(path), Ok(bytes)) = (path_of(key), picture) else {
            continue;
        };
        let file = path.resolve(root);
        if !file.is_file() {
            write(&file, bytes)?;
        }
    }
    Ok(())
}

/// What became of one still once its job finished.
fn land(
    project: &mut Project,
    root: &Path,
    id: &AssetId,
    answers: &[(String, Result<Vec<u8>, String>)],
) -> Result<Outcome, ImageError> {
    let brief = project
        .asset(id)
        .map(|asset| Brief::of(project, root, asset));
    let brief = match brief {
        Some(Ok(brief)) => brief,
        Some(Err(why)) => return Ok(released(project, id, why.to_string())),
        None => return Err(ImageError::NoSuchAsset { id: id.clone() }),
    };
    if brief.realized(root) {
        let output = brief.output();
        let bytes = std::fs::read(output.resolve(root)).unwrap_or_default();
        record(project, id, &output, &bytes, half(&brief)?);
        return Ok(Outcome::Collected {
            path: output,
            bytes: bytes.len(),
        });
    }
    let key = brief.key();
    let message = match answers.iter().find(|(sent, _)| *sent == key) {
        Some((_, Err(why))) => why.clone(),
        Some((_, Ok(_))) => String::from("its picture could not be kept"),
        None => String::from(
            "its brief changed while it waited; the picture drawn from the old one is kept in \
             generated/ and costs nothing if that brief comes back",
        ),
    };
    Ok(released(project, id, message))
}

/// [`release`], reported as a refusal in `message`'s words.
fn released(project: &mut Project, id: &AssetId, message: String) -> Outcome {
    release(project, id);
    Outcome::Failed { message }
}

/// Takes the ticket away and puts the state back: stale when an older drawing
/// is still on the asset, a sketch otherwise — so the still is ordered again
/// only when somebody asks, never by a sweep.
fn release(project: &mut Project, id: &AssetId) {
    if let Some(asset) = project.assets.iter_mut().find(|asset| &asset.id == id) {
        asset.operation = None;
        asset.state = Some(if asset.path.is_some() {
            GenerationState::Stale
        } else {
            GenerationState::Sketch
        });
    }
}

/// Every batch a still waits in, once each, in document order.
fn operations(project: &Project) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for asset in &project.assets {
        if asset.kind != AssetKind::GeneratedImage {
            continue;
        }
        if let Some(operation) = &asset.operation
            && !seen.contains(operation)
        {
            seen.push(operation.clone());
        }
    }
    seen
}

/// The stills waiting in `operation`.
fn members(project: &Project, operation: &str) -> Vec<AssetId> {
    project
        .assets
        .iter()
        .filter(|asset| asset.kind == AssetKind::GeneratedImage)
        .filter(|asset| asset.operation.as_deref() == Some(operation))
        .map(|asset| asset.id.clone())
        .collect()
}

/// Where a key's picture lands, or `None` for a key that could not have been
/// one of ours — it came back over the network, and must not name a path
/// outside `generated/`.
fn path_of(key: &str) -> Option<ProjectPath> {
    let plain = !key.is_empty()
        && !key.starts_with('.')
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    plain.then(|| ProjectPath::new(format!("{GENERATED_DIR}/{key}.jpg")))
}

#[cfg(test)]
mod tests {
    use super::path_of;

    #[test]
    fn only_a_plain_name_lands_and_only_in_generated() {
        assert_eq!(
            path_of("poster-abc").map(|path| path.to_string()),
            Some(String::from("generated/poster-abc.jpg"))
        );
        assert_eq!(path_of("../escape"), None);
        assert_eq!(path_of("a/b"), None);
        assert_eq!(path_of(""), None);
    }
}
