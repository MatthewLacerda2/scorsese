//! Where `println!` goes, and what happens when nobody is reading any more.
//!
//! An agent pipes a command into `head`, `tail` or `grep` all the time, and
//! `head -1` closes the pipe after one line. The standard `println!` panics on
//! that write — "failed printing to stdout: Broken pipe" and exit 101 — which a
//! caller reads as "the command failed" when it finished (#996). Unix tools stop
//! quietly when their reader goes away, and so does scorsese.
//!
//! The fix is one place rather than one per print: [`println!`] and [`print!`]
//! are shadowed for the whole crate by the two macros below (`lib.rs` declares
//! this module first, with `#[macro_use]`, so every module after it sees them).
//! A command keeps writing `println!` exactly as before, a new one gets the same
//! behaviour without knowing it, and [`until_reader_leaves`] around the dispatch
//! turns "the reader left" into an ordinary, silent success.
//!
//! How it stops is by unwinding, not by exiting on the spot: the write that
//! notices the closed pipe calls [`std::panic::resume_unwind`], which skips the
//! panic hook — so nothing is printed — but still runs every destructor on the
//! way out, the way a real return would. Any other failed write still panics
//! with the standard message: a full disk is a failure, not a reader leaving.
//!
//! Not covered, on purpose: stderr, and code that writes to `std::io::stdout()`
//! itself and returns the error — that one fails the ordinary way, with an
//! `error:` line.

use std::fmt;
use std::io::{ErrorKind, Write};
use std::panic::{self, AssertUnwindSafe};

/// The crate's `println!`: the standard one, except that a closed stdout stops
/// the command quietly instead of panicking.
macro_rules! println {
    () => {
        $crate::out::print(format_args!("\n"))
    };
    ($($argument:tt)*) => {
        $crate::out::print(format_args!("{}\n", format_args!($($argument)*)))
    };
}

/// The crate's `print!`, with the same difference as its `println!`.
macro_rules! print {
    ($($argument:tt)*) => {
        $crate::out::print(format_args!($($argument)*))
    };
}

/// What unwinds out of a write to a stdout whose reader has gone. Private, so
/// nothing but [`print`] can raise it and nothing but [`until_reader_leaves`]
/// mistakes another panic for it.
struct ReaderGone;

/// Writes to stdout, the way the standard `print!` does.
pub(crate) fn print(text: fmt::Arguments<'_>) {
    if let Err(error) = std::io::stdout().lock().write_fmt(text) {
        if error.kind() == ErrorKind::BrokenPipe {
            panic::resume_unwind(Box::new(ReaderGone));
        }
        panic!("failed printing to stdout: {error}");
    }
}

/// Runs `command`, treating a reader that left as the command having finished.
///
/// Every other panic carries on unwinding untouched, so it still reaches `main`
/// the way it always did.
pub(crate) fn until_reader_leaves(
    command: impl FnOnce() -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    match panic::catch_unwind(AssertUnwindSafe(command)) {
        Ok(outcome) => outcome,
        Err(payload) if payload.is::<ReaderGone>() => Ok(()),
        Err(payload) => panic::resume_unwind(payload),
    }
}
