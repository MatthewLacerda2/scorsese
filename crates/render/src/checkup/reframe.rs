//! Changing a project's shape, and saying what is left to settle (#1278).
//!
//! [`scorsese_core::frame::refit`] carries the change out — never refused —
//! and re-fits footage that filled the old frame. What it cannot see without a
//! layout, this reads off one at the new shape: a title that now wraps past
//! its band, a clip that now leaves the frame, worded exactly as the checkup
//! words them (#1096), for the agent to settle.

use std::fmt;
use std::path::Path;

use scorsese_core::frame::{Refit, refit};
use scorsese_core::{Project, Ratio};

use super::framed;

/// What a change of shape did, and what it left for somebody to decide.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reframed {
    /// What the re-fit changed.
    pub refit: Refit,
    /// Every cut title and every clip off the frame at the new shape, already
    /// worded. A finding the old shape had too is here as well: it is still
    /// true, and the reader is deciding about the frame as it is now.
    pub unsettled: Vec<String>,
}

/// Reshapes `project` to `to` and reads its layout at the new shape. Nothing
/// is saved: the caller writes the document the way it writes every edit.
pub fn reframe(project: &mut Project, project_dir: &Path, to: Ratio) -> Reframed {
    let refit = refit(project, to);
    let unsettled = if refit.changed() {
        framed::off_frame(project, project_dir)
    } else {
        Vec::new()
    };
    Reframed { refit, unsettled }
}

impl fmt::Display for Reframed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Refit { from, to, filled } = &self.refit;
        if !self.refit.changed() {
            return write!(f, "The project is already {to}; nothing changed.");
        }
        write!(f, "Reframed from {from} to {to}.")?;
        if !filled.is_empty() {
            let named: Vec<String> = filled.iter().map(|clip| format!("`{clip}`")).collect();
            write!(
                f,
                "\n  {} that filled the old frame {} the new one (fit → fill): {}",
                if filled.len() == 1 {
                    "1 clip".to_owned()
                } else {
                    format!("{} clips", filled.len())
                },
                if filled.len() == 1 { "fills" } else { "fill" },
                named.join(", ")
            )?;
        }
        if self.unsettled.is_empty() {
            return write!(f, "\n  Nothing is cut or off the frame at {to}.");
        }
        write!(f, "\n  Left to settle at {to}:")?;
        for line in &self.unsettled {
            write!(f, "\n  - {line}")?;
        }
        Ok(())
    }
}
