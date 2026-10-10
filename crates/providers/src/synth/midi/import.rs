//! A MIDI file in; a song recipe, and the asset that points at it, out.
//!
//! The conversion itself is `scorsese-zimmer`'s, because reading bytes into a
//! song is arithmetic on a document. What lives here is the half that touches
//! a disk — reading the file named, writing the recipe where `synth new` would
//! — and the words both front doors report the result in.

use std::path::Path;

use scorsese_core::{AssetId, Project};
use scorsese_zimmer::midi::{self, BARS_PER_PATTERN};
use scorsese_zimmer::song::PatternEntry;

use super::super::create::start;
use super::super::error::SynthesisError;
use super::super::recipe::Recipe;

/// What an import made, and what it could not carry.
#[derive(Debug, Clone, PartialEq)]
pub struct FromMidi {
    /// The new `synth_audio` asset.
    pub id: AssetId,
    /// How many song tracks the file's parts became.
    pub tracks: usize,
    /// How many notes, across all of them.
    pub notes: usize,
    /// How many patterns the bars were cut into.
    pub patterns: usize,
    /// The opening tempo.
    pub bpm: f32,
    /// How many times the tempo changes after it.
    pub tempo_changes: usize,
    /// The key the file declared, if it declared one.
    pub key: Option<String>,
    /// One sentence per kind of thing the file carried and the song does not.
    pub left_out: Vec<String>,
}

impl FromMidi {
    /// The report, in lines, shared by the command line and the MCP tool so
    /// the two cannot describe one import differently.
    ///
    /// What was left out comes after the counts and before the reminder,
    /// because it is the part a reader acts on: an import that sounds drier
    /// than the file is explained by a line here, or not at all.
    pub fn lines(&self) -> Vec<String> {
        let mut shape = format!(
            "{} track(s), {} notes, {} pattern(s) of up to {BARS_PER_PATTERN} bars, at {} bpm",
            self.tracks, self.notes, self.patterns, self.bpm
        );
        if self.tempo_changes > 0 {
            shape.push_str(&format!(" with {} tempo change(s)", self.tempo_changes));
        }
        if let Some(key) = &self.key {
            shape.push_str(&format!(", in {key}"));
        }
        let mut lines = vec![shape];
        lines.extend(self.left_out.iter().map(|why| format!("left out: {why}")));
        lines.push(
            "every track plays a plain default patch, a placeholder to replace — the notes \
             are the file's, the sounds are not"
                .to_owned(),
        );
        lines
    }
}

/// Reads the Standard MIDI File at `file` into a song recipe in `recipes/`,
/// and adds the `synth_audio` asset that points at it, in `sketch`.
///
/// Named `name` when one is given, and after the file otherwise — `song.mid`
/// becomes `song` — suffixed out of the way the way `synth new` is, so an
/// import never overwrites a recipe already there. The project is left
/// describing the new asset; saving it is the caller's.
pub fn import_midi(
    project: &mut Project,
    project_root: &Path,
    file: &Path,
    name: Option<&str>,
) -> Result<FromMidi, SynthesisError> {
    let bytes = std::fs::read(file).map_err(|why| SynthesisError::MidiRead {
        path: file.to_path_buf(),
        why,
    })?;
    let imported = midi::import(&bytes).map_err(|why| SynthesisError::Midi {
        path: file.to_path_buf(),
        why,
    })?;
    let stem = file
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned());
    let name = name
        .map(str::to_owned)
        .or(stem)
        .unwrap_or_else(|| "midi".to_owned());

    let song = imported.song;
    let notes = song
        .patterns
        .values()
        .flat_map(|pattern| &pattern.notes)
        .filter(|entry| matches!(entry, PatternEntry::Note(_)))
        .count();
    let (tracks, patterns, bpm) = (song.tracks.len(), song.patterns.len(), song.bpm);
    let (tempo_changes, key) = (song.tempo.len(), song.key.clone());
    let id = start(project, project_root, &name, &Recipe::Song(song))?;
    Ok(FromMidi {
        id,
        tracks,
        notes,
        patterns,
        bpm,
        tempo_changes,
        key,
        left_out: imported.left_out,
    })
}
