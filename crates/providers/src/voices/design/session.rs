//! One design, and the three samples it paid for, kept on disk.
//!
//! # Why `generated/` and not `cache/`
//!
//! The project format draws exactly this line: `cache/` is rebuildable and
//! gitignored, `generated/` holds output somebody paid for and deleting it
//! loses money. Three MP3s that cost real cents are unambiguously the second
//! kind, however throwaway they feel — two of them will be discarded by a
//! person, which is not the same as being free to discard by a program.
//!
//! Putting them there buys the property that matters: **the samples are named
//! for the hash of the brief**, so asking for the same design twice reads them
//! off disk instead of billing the passage again. That is the same guarantee a
//! generated shot has, arrived at the same way, and it is what makes a dropped
//! connection or a repeated command cost nothing.
//!
//! # The record beside them is the load-bearing half
//!
//! An MP3 does not carry the `generated_voice_id` that turns it into a voice,
//! and that id is what the second call needs. So each design writes a small
//! JSON beside its samples holding the brief, the estimate, and the three ids
//! in the order they came back. Without it the samples are three sounds nobody
//! can act on — audible, paid for, and useless.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use scorsese_core::{GENERATED_DIR, ProjectPath, Timestamp};

use super::brief::Brief;
use super::error::DesignError;
use super::price::Estimate;
use super::studio::Candidate;

/// Where designs live inside `generated/`.
///
/// A directory of its own because one design is four files, and a flat
/// `generated/` shared with the shots would make the shots hard to read.
const DESIGN_DIR: &str = "voice-design";

/// One candidate, as it is on disk.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sample {
    /// The vendor's handle, and what [`keep`](super::keep) is given.
    pub generated_voice_id: String,
    /// Where the MP3 is, project-relative — so the record survives `scp -r`
    /// along with everything else in the project.
    pub path: ProjectPath,
    /// How long it runs, where the vendor said.
    #[serde(default)]
    pub seconds: Option<f64>,
}

/// One design: what was asked, what it cost, and what came back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    /// The fingerprint of the brief, which is also the directory name. Written
    /// into the file as well so a record moved out of its directory still says
    /// which design it is.
    pub digest: String,
    /// The description the candidates were designed from.
    pub prompt: String,
    /// The passage they read, and the only thing that was billed.
    pub passage: String,
    /// The determinism knob, where one was given.
    #[serde(default)]
    pub seed: Option<u32>,
    /// How literally the description was to be followed, where that was set.
    #[serde(default)]
    pub guidance: Option<f64>,
    /// What we worked out this cost. **Never a bill** — no ElevenLabs reply
    /// reports what a call cost.
    pub estimated_cost_cents: u64,
    /// When it was designed, as a person reads a date.
    #[serde(default)]
    pub designed_at: Option<Timestamp>,
    /// The candidates, in the order they came back.
    pub candidates: Vec<Sample>,
}

impl Session {
    /// The candidate ids other than `chosen`.
    ///
    /// What the create call sends as `played_not_selected_voice_ids`: the ones
    /// that were auditioned and passed over. It costs nothing to send and the
    /// vendor uses it to improve the model, so the only way to get it wrong is
    /// to forget it.
    pub fn passed_over(&self, chosen: &str) -> Vec<String> {
        self.candidates
            .iter()
            .map(|sample| sample.generated_voice_id.clone())
            .filter(|id| id != chosen)
            .collect()
    }

    /// Whether this design offered `chosen`.
    pub fn offered(&self, chosen: &str) -> bool {
        self.candidates
            .iter()
            .any(|sample| sample.generated_voice_id == chosen)
    }
}

/// Where a design's record lives, inside `root`.
fn record(root: &Path, digest: &str) -> PathBuf {
    directory(root, digest).join("design.json")
}

/// Where a design's files live, inside `root`.
fn directory(root: &Path, digest: &str) -> PathBuf {
    root.join(GENERATED_DIR).join(DESIGN_DIR).join(digest)
}

/// The design already paid for under this fingerprint, if there is a readable
/// one.
///
/// A record whose samples have gone is **not** a hit: the whole value of
/// finding one is being able to listen again, and answering with three paths to
/// files that are not there would report a design as cached and hand back
/// silence.
pub(super) fn read(root: &Path, digest: &str) -> Option<Session> {
    let text = std::fs::read_to_string(record(root, digest)).ok()?;
    let session: Session = serde_json::from_str(&text).ok()?;
    session
        .candidates
        .iter()
        .all(|sample| sample.path.resolve(root).is_file())
        .then_some(session)
}

/// The design in this project that offered `generated_voice_id`.
///
/// A scan rather than an index, because there are as many of these as somebody
/// has designed voices — a handful — and an index is another file that can
/// disagree with the directory it describes.
pub(super) fn find(root: &Path, generated_voice_id: &str) -> Option<Session> {
    let designs = std::fs::read_dir(root.join(GENERATED_DIR).join(DESIGN_DIR)).ok()?;
    designs
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter_map(|digest| read(root, &digest))
        .find(|session| session.offered(generated_voice_id))
}

/// Writes a design and its samples, and answers with the record.
///
/// **Every failure here is fatal**, which is what separates this from a voice
/// listing's cache: a listing that will not save re-reads for free, and these
/// are bytes somebody has already been billed for. Losing them silently would
/// mean paying twice for the same three samples and never knowing why.
pub(super) fn write(
    root: &Path,
    brief: &Brief,
    candidates: &[Candidate],
    estimate: Estimate,
) -> Result<Session, DesignError> {
    let digest = brief.digest();
    let folder = directory(root, &digest);
    std::fs::create_dir_all(&folder).map_err(|source| DesignError::Write {
        doing: format!("creating {}", folder.display()),
        source,
    })?;

    let mut samples = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.iter().enumerate() {
        // Numbered as well as named, so the three files sort into the order
        // they came back in — which is the order every surface lists them in,
        // and the order somebody will refer to them by out loud.
        let path = ProjectPath::new(format!(
            "{GENERATED_DIR}/{DESIGN_DIR}/{digest}/{}-{}.mp3",
            index + 1,
            candidate.generated_voice_id
        ));
        let file = path.resolve(root);
        // Atomically, for the reason every write in this crate is: a truncated
        // sample is not a crash but something worse — a file that exists, so
        // the design reads as cached, and plays as noise.
        scorsese_core::write::atomically(&file, &candidate.sample).map_err(|source| {
            DesignError::Write {
                doing: format!("writing {}", file.display()),
                source,
            }
        })?;
        samples.push(Sample {
            generated_voice_id: candidate.generated_voice_id.clone(),
            path,
            seconds: candidate.seconds,
        });
    }

    let session = Session {
        digest,
        prompt: brief.prompt.clone(),
        passage: brief.passage.clone(),
        seed: brief.seed,
        guidance: brief.guidance,
        estimated_cost_cents: estimate.cents,
        designed_at: Timestamp::now(),
        candidates: samples,
    };
    let file = record(root, &session.digest);
    let text = serde_json::to_string_pretty(&session).map_err(|error| DesignError::Write {
        doing: String::from("describing the design"),
        source: std::io::Error::other(error.to_string()),
    })?;
    scorsese_core::write::atomically(&file, text.as_bytes()).map_err(|source| {
        DesignError::Write {
            doing: format!("writing {}", file.display()),
            source,
        }
    })?;
    Ok(session)
}
