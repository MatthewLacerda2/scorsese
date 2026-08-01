//! What the project says about itself: its script, and every note on it.
//!
//! A description says what the cut *contains*, and this is now part of what it
//! contains. The reasoning behind an edit used to live in a chat log and die
//! with it; if the project is going to carry it, the command whose whole job is
//! reading a project back has to say so, or it is carried where nobody looks.
//!
//! **This is derived from the document, not from the plan**, which is why it is
//! its own type rather than more fields on [`super::Description`]. A
//! description is a function of a [`crate::Plan`] — a range of it, at an output
//! framerate — and a note is none of those things: it belongs to the element
//! whatever range is being looked at, and rendering half the film does not
//! make half of it true.
//!
//! **Listed once, not repeated per stretch.** A clip that spans a minute
//! appears in every stretch of that minute, and hanging its note off each of
//! them would print a paragraph forty times and bury the cut it was meant to
//! explain. So the notes are a block of their own, in document order.

use std::fmt;

use scorsese_core::{Noted, Project, ProjectPath};

/// The writing a project carries about itself.
///
/// Both halves are optional and independent: a project may have a script and no
/// notes, notes and no script, or — the ordinary state of a fresh project —
/// neither, in which case there is nothing to print and [`Commentary::is_empty`]
/// says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commentary<'a> {
    /// Where the script lives, as the document names it. The path only: this
    /// module never opens the file, because it never parses it either — see
    /// [`Project::script`].
    pub script: Option<&'a ProjectPath>,
    /// Every note, in reading order: assets, then each track with its clips.
    pub notes: Vec<Noted<'a>>,
}

impl<'a> Commentary<'a> {
    /// Reads a project's script path and notes off the document.
    pub fn of(project: &'a Project) -> Self {
        Self {
            script: project.script.as_ref(),
            notes: project.notes().collect(),
        }
    }

    /// True when the project explains nothing about itself.
    pub fn is_empty(&self) -> bool {
        self.script.is_none() && self.notes.is_empty()
    }
}

impl fmt::Display for Commentary<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut written = false;
        if let Some(script) = self.script {
            // Said as an instruction rather than as a field, because the useful
            // thing about a script is that it is there to be read — an agent
            // told `script: script.md` and nothing else has been told a fact,
            // not handed a next step.
            write!(f, "script: {script} — read it before changing the cut")?;
            written = true;
        }
        for note in &self.notes {
            if written {
                f.write_str("\n")?;
            }
            write!(f, "note on {note}")?;
            written = true;
        }
        Ok(())
    }
}
