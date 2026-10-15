//! What the command line asks of the window (#1204).
//!
//! Parsed before any window exists, so `--help` answers on a terminal instead
//! of opening a window that blocks the shell, and a flag nobody knows is
//! refused instead of being opened as the path of a project.

use std::path::PathBuf;

/// What the binary was asked to do.
#[derive(Debug, PartialEq, Eq)]
pub enum Launch {
    /// Open the window, on this project if a path was given.
    Window(Option<PathBuf>),
    /// Print this text and exit.
    Say(String),
    /// Refuse with this text and exit with a failure.
    Refuse(String),
}

/// The usage text.
const USAGE: &str =
    "scorsese-app — the desktop window for previewing and tweaking a scorsese project

Usage: scorsese-app [PROJECT.scor]

  PROJECT.scor   a project folder to open at start-up; without one, the window
                 opens empty and offers a file dialog
  -h, --help     print this and exit";

/// Reads the arguments after the program's name.
pub fn launch(args: impl IntoIterator<Item = String>) -> Launch {
    let mut path = None;
    for arg in args {
        if arg == "--help" || arg == "-h" {
            return Launch::Say(USAGE.to_owned());
        }
        if arg.starts_with('-') {
            return Launch::Refuse(format!("unknown option `{arg}`\n\n{USAGE}"));
        }
        if path.replace(PathBuf::from(arg)).is_some() {
            return Launch::Refuse(format!("only one project can be opened\n\n{USAGE}"));
        }
    }
    Launch::Window(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(args: &[&str]) -> Launch {
        launch(args.iter().map(|arg| (*arg).to_owned()))
    }

    /// Help answers, whatever else is on the line, and opens nothing.
    #[test]
    fn help_is_answered_and_not_opened() {
        for args in [&["--help"][..], &["-h"], &["a.scor", "--help"]] {
            assert!(matches!(read(args), Launch::Say(text) if text.contains("Usage")));
        }
    }

    /// A flag nobody knows is refused rather than opened as a project.
    #[test]
    fn an_unknown_flag_is_refused() {
        assert!(matches!(read(&["--frobnicate"]), Launch::Refuse(_)));
        assert!(matches!(read(&["a.scor", "b.scor"]), Launch::Refuse(_)));
    }

    /// A path is still the development convenience it was.
    #[test]
    fn a_path_or_nothing_opens_the_window() {
        assert_eq!(read(&[]), Launch::Window(None));
        assert_eq!(
            read(&["a.scor"]),
            Launch::Window(Some(PathBuf::from("a.scor")))
        );
    }
}
