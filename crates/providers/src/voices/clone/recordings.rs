//! The recordings a clone is made from: a project's own audio assets, read
//! off disk and named for what they are.

use std::collections::HashSet;
use std::hash::Hash;
use std::path::Path;

use scorsese_core::{Asset, AssetId, AssetKind, Project, ProjectPath};

use super::error::CloneError;
use super::ledger::Source;
use super::{
    ENOUGH_SECONDS, MAX_RECORDING_BYTES, MAX_RECORDING_SECONDS, MAX_RECORDINGS,
    RECOMMENDED_SECONDS, Recording,
};

/// What a clone sends, and what the ledger says it was sent from.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Gathered {
    /// The recordings, in the order they were named.
    pub(super) recordings: Vec<Recording>,
    /// Where each came from.
    pub(super) sources: Vec<Source>,
    /// How long they run together, where every one was probed.
    pub(super) seconds: Option<f64>,
}

/// The assets `ids` of `project`, at `root`, as recordings — refused unless
/// each is an imported audio asset whose file can be read, and unless together
/// they are no more than a clone takes ([`too_much`]), which is asked before
/// any file is read. An id named twice, or two assets of one file (one path,
/// or one content hash), is one recording.
pub(super) fn gather(
    root: &Path,
    project: &Project,
    ids: &[AssetId],
) -> Result<Gathered, CloneError> {
    let ids = distinct(ids);
    if ids.is_empty() {
        return Err(CloneError::NoRecordings);
    }
    let mut named = Vec::with_capacity(ids.len());
    let mut seconds = Some(0.0);
    let mut bytes = 0u64;
    for id in &ids {
        let asset = project
            .asset(id)
            .ok_or_else(|| CloneError::NoSuchAsset { asset: id.clone() })?;
        let path = match (&asset.kind, &asset.path) {
            (AssetKind::Audio, Some(path)) => path,
            (kind, _) => {
                return Err(CloneError::NotARecording {
                    asset: id.clone(),
                    kind: format!("a {kind} asset"),
                });
            }
        };
        let same = |(_, other, at): &(&AssetId, &Asset, &ProjectPath)| {
            *at == path || (asset.sha256.is_some() && other.sha256 == asset.sha256)
        };
        if named.iter().any(same) {
            continue;
        }
        let more = asset
            .media
            .as_ref()
            .and_then(|media| media.duration_seconds);
        seconds = seconds.zip(more).map(|(sum, more): (f64, f64)| sum + more);
        // What the file says it holds, where it can be asked without reading
        // it; a file that cannot be found is refused when it is read.
        bytes += path
            .resolve(root)
            .ok()
            .and_then(|file| std::fs::metadata(file).ok())
            .map_or(0, |found| found.len());
        named.push((id, asset, path));
    }
    if let Some(why) = too_much(named.len(), bytes, seconds) {
        return Err(CloneError::TooMuch(why));
    }
    let mut gathered = Gathered {
        recordings: Vec::with_capacity(named.len()),
        sources: Vec::with_capacity(named.len()),
        seconds,
    };
    for (id, asset, path) in named {
        let unreadable = |source| CloneError::Unreadable {
            path: path.to_string(),
            source,
        };
        let file = path
            .resolve(root)
            .map_err(|refused| unreadable(refused.into()))?;
        let bytes = std::fs::read(file).map_err(unreadable)?;
        let file_name = path.as_str().rsplit('/').next().unwrap_or(path.as_str());
        gathered.recordings.push(Recording {
            file_name: file_name.to_owned(),
            media_type: media_type(file_name).to_owned(),
            bytes,
        });
        gathered.sources.push(Source {
            asset: (*id).clone(),
            path: path.clone(),
            sha256: asset.sha256.clone(),
        });
    }
    Ok(gathered)
}

/// `ids` in the order first named, each once: a recording named twice is
/// sent once. Linear in `ids`, however many repeat.
pub fn distinct<T: Clone + Eq + Hash>(ids: &[T]) -> Vec<T> {
    let mut seen: HashSet<&T> = HashSet::with_capacity(ids.len());
    ids.iter().filter(|id| seen.insert(*id)).cloned().collect()
}

/// Why `count` recordings of `bytes` together, running `seconds` where every
/// one was probed, are more than a clone is made from — [`MAX_RECORDINGS`],
/// [`MAX_RECORDING_BYTES`], [`MAX_RECORDING_SECONDS`] — or `None` when they
/// are not. Asked before a file is read, on every surface.
pub fn too_much(count: usize, bytes: u64, seconds: Option<f64>) -> Option<String> {
    let enough = "ElevenLabs says more than two or three minutes of speech adds little";
    if count > MAX_RECORDINGS {
        return Some(format!(
            "{count} recordings is more than a clone is made from: at most {MAX_RECORDINGS}. \
             {enough}, so pick the clearest. Nothing was sent."
        ));
    }
    if bytes > MAX_RECORDING_BYTES {
        return Some(format!(
            "the recordings come to {} MB, more than the {} MB a clone is made from. {enough}, \
             so send less of it, or as MP3. Nothing was sent.",
            bytes.div_ceil(1024 * 1024),
            MAX_RECORDING_BYTES / (1024 * 1024)
        ));
    }
    match seconds {
        Some(seconds) if seconds > MAX_RECORDING_SECONDS => Some(format!(
            "the recordings run {:.1} minutes, more than the {:.0} a clone is made from. \
             {enough}, so send less of it. Nothing was sent.",
            seconds / 60.0,
            MAX_RECORDING_SECONDS / 60.0
        )),
        _ => None,
    }
}

/// What an audio file is, by its name — what the form says each part is.
pub fn media_type(file_name: &str) -> &'static str {
    let extension = file_name
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "m4a" | "mp4" | "aac" => "audio/mp4",
        "ogg" | "oga" | "opus" => "audio/ogg",
        "flac" => "audio/flac",
        "webm" => "audio/webm",
        _ => "application/octet-stream",
    }
}

/// The vendor's advice on how much to record, when `seconds` is outside it —
/// a note beside a clone that worked, never a refusal: the vendor sets no
/// minimum, and a short clone is still a voice.
pub fn advice(seconds: Option<f64>) -> Option<String> {
    let seconds = seconds?;
    if seconds < *RECOMMENDED_SECONDS.start() {
        Some(format!(
            "The recordings run about {seconds:.0} seconds; ElevenLabs recommends one to two \
             minutes of clear speech, so a longer recording, cloned again, should sound more \
             like the speaker."
        ))
    } else if seconds > ENOUGH_SECONDS {
        Some(format!(
            "The recordings run about {seconds:.0} seconds; ElevenLabs says more than three \
             minutes yields little improvement, so there is no need to record more."
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_named_for_its_extension() {
        for (file, media) in [
            ("take.MP3", "audio/mpeg"),
            ("assets/a.wav", "audio/wav"),
            ("a.m4a", "audio/mp4"),
            ("a.aac", "audio/mp4"),
            ("a.opus", "audio/ogg"),
            ("a.flac", "audio/flac"),
            ("a.webm", "audio/webm"),
            ("noext", "application/octet-stream"),
        ] {
            assert_eq!(media_type(file), media, "{file}");
        }
    }

    #[test]
    fn advice_is_only_given_outside_what_the_vendor_recommends() {
        assert!(advice(Some(20.0)).is_some_and(|said| said.contains("one to two")));
        assert_eq!(advice(Some(90.0)), None);
        assert_eq!(advice(Some(60.0)), None, "a minute is inside it");
        assert_eq!(advice(Some(180.0)), None, "three minutes is inside it");
        assert!(advice(Some(400.0)).is_some_and(|said| said.contains("little improvement")));
        assert_eq!(advice(None), None);
    }
}
