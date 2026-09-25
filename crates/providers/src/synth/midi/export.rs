//! A song recipe out as a Standard MIDI File.

use std::path::{Path, PathBuf};

use scorsese_core::{AssetId, CACHE_DIR, Project, ProjectPath};
use scorsese_zimmer::midi::{self, Drum};

use super::super::error::SynthesisError;
use super::super::recipe::Recipe;
use super::super::{read_recipe, write};

/// Where an export lands when the caller does not say — under `cache/`,
/// because the file is rebuildable from the recipe at any time, and a copy
/// kept beside it would go stale the first time the recipe is edited.
const EXPORT_DIR: &str = "midi";

/// What an export wrote, and what it could not carry.
#[derive(Debug, Clone, PartialEq)]
pub struct ToMidi {
    /// The file that was written.
    pub file: PathBuf,
    /// The same path as a report should say it: project-relative when it
    /// landed in the project's own `cache/`, and as the caller wrote it when
    /// the caller chose.
    pub shown: String,
    /// How big it is.
    pub bytes: usize,
    /// How many song tracks became MIDI tracks.
    pub tracks: usize,
    /// How many notes were written.
    pub notes: usize,
    /// How many tempo events the file holds, ramps' steps included.
    pub tempo_events: usize,
    /// The tracks written on the drum channel, as they were asked for.
    pub drums: Vec<Drum>,
    /// One sentence per kind of thing the song played and the file does not.
    pub left_out: Vec<String>,
}

impl ToMidi {
    /// The report, in lines, shared by the command line and the MCP tool.
    ///
    /// The reminder about sounds comes last and always, for the reason the
    /// import's does: it is the one thing every export leaves behind, and the
    /// first thing someone opening the file in a DAW would otherwise wonder.
    pub fn lines(&self) -> Vec<String> {
        let mut shape = format!(
            "{} track(s), {} notes, {} tempo event(s), {} bytes",
            self.tracks, self.notes, self.tempo_events, self.bytes
        );
        if !self.drums.is_empty() {
            let named: Vec<String> = self.drums.iter().map(ToString::to_string).collect();
            shape.push_str(&format!(", drums on channel 10: {}", named.join(", ")));
        }
        let mut lines = vec![shape];
        lines.extend(self.left_out.iter().map(|why| format!("left out: {why}")));
        lines.push(
            "the file is the notes and the tempo; the sounds — patches, effects, gain, pan, \
             automation — stay in the recipe, and every track is left on the DAW's default \
             instrument"
                .to_owned(),
        );
        lines
    }
}

/// Writes the song recipe behind asset `id` as a Standard MIDI File, at `out`
/// or at `cache/midi/<id>.mid`, with `drums` on the percussion channel.
///
/// The project is read and never written: an export is a view of the recipe,
/// not a new asset, and nothing in the project points at it.
///
/// **What asserts this lives in `crates/cli`**, as `super::super::partial`'s
/// does and for the same reason: every claim here is about a file on disk,
/// and the conversion itself is tested where it lives, in `scorsese-zimmer`.
pub fn export_midi(
    project: &Project,
    project_root: &Path,
    id: &AssetId,
    drums: &[Drum],
    out: Option<&Path>,
) -> Result<ToMidi, SynthesisError> {
    let asset = project
        .asset(id)
        .ok_or_else(|| SynthesisError::NoSuchAsset { id: id.clone() })?;
    if !asset.kind.is_synthesized() {
        return Err(SynthesisError::NotSynthesised { id: id.clone() });
    }
    let (recipe, _, _) = read_recipe(asset, project_root)?;
    let Recipe::Song(song) = &recipe else {
        return Err(SynthesisError::NoScore { id: id.clone() });
    };
    let exported = midi::export(song, drums).map_err(|why| SynthesisError::MidiExport {
        id: id.clone(),
        why,
    })?;

    let (destination, shown) = match out {
        Some(path) => (path.to_path_buf(), path.display().to_string()),
        None => {
            let relative = ProjectPath::new(format!("{CACHE_DIR}/{EXPORT_DIR}/{id}.mid"));
            (relative.resolve(project_root), relative.to_string())
        }
    };
    write(&destination, &exported.bytes)?;
    Ok(ToMidi {
        file: destination,
        shown,
        bytes: exported.bytes.len(),
        tracks: exported.tracks,
        notes: exported.notes,
        tempo_events: exported.tempo_events,
        drums: drums.to_vec(),
        left_out: exported.left_out,
    })
}
