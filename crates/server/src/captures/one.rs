//! `scorsese-server capture-one`: one capture, in a process of its own.
//!
//! The worker ([`super::worker`]) runs this for each page, so that the
//! deadline can kill the capture and the browser it started as one process
//! group. It is [`scorsese_render::page::capture_frames`] and nothing else,
//! following the project's links into the owner's library: the frames land in
//! the project's `cache/`, and what went wrong is its last line on stderr,
//! which the worker hands back as the reason.
//!
//! **Only the frames asked for are drawn** (#890), the stretch the render
//! shows: the page's clock runs through the ones before it without drawing
//! them, as it does locally (#809). **In one browser**, where locally a long
//! stretch is drawn by up to four at once: the capture container is sized for
//! one (#773), and running ahead, which needs no second, is most of the win.
//!
//! **A long stretch is a part a turn** (#1708): the worker names one part of
//! it (`--frames`, one of `Asked::parts`), which is captured as a piece of
//! the cache, and the last part joins them all into the one capture
//! ([`scorsese_render::page::capture_part`]).

use std::ops::Range;
use std::path::PathBuf;

use scorsese_render::Tools;
use scorsese_render::page::{self, Chrome};

use super::{Ask, read};

/// `scorsese-server capture-one`: the worker's, never a person's.
#[derive(Debug, clap::Args)]
pub struct Args {
    /// The project folder the page is in.
    #[arg(long)]
    pub project: PathBuf,
    /// The job's `ask.json`.
    #[arg(long)]
    pub ask: PathBuf,
    /// Which of its requests to capture, from 0.
    #[arg(long)]
    pub index: usize,
    /// The part of its frames to capture this turn, `START..END`: one of
    /// `Asked::parts`. Without it, every frame asked for at once.
    #[arg(long, value_parser = frames)]
    pub frames: Option<Range<u64>>,
    /// Run the browser without its sandbox (`capture-worker --no-sandbox`).
    #[arg(long)]
    pub no_sandbox: bool,
    /// A folder outside the project its links may lead into: the owner's
    /// library, which the server links each media file from (#857). Any link
    /// leading anywhere else is refused, as it is locally.
    #[arg(long)]
    pub follow: Vec<PathBuf>,
}

/// Captures the request `args` names into its project's cache.
pub fn run(args: &Args) -> Result<(), String> {
    let ask: Ask = read(&args.ask).ok_or("the job's ask.json could not be read")?;
    let asked = ask
        .requests
        .get(args.index)
        .ok_or("the job asks for no such capture")?;
    let (request, frames) = (asked.request()?, asked.frames()?);
    let tools = Tools::discover().map_err(|error| error.to_string())?;
    let chrome = Chrome::discover().map_err(|error| error.to_string())?;
    let chrome = if args.no_sandbox {
        chrome.unsandboxed()
    } else {
        chrome.sandboxed()
    };
    // The server laid the project out, its `cache/` a link to the project's
    // page cache in the worker's spool (`dispatch::share_cache`), and a mount
    // in the launcher's container: either way its own (#1718).
    let laid = page::Laid {
        follow: &args.follow,
        cache: true,
    };
    let Some(part) = &args.frames else {
        return page::capture_frames(&chrome, &tools, &args.project, laid, &request, frames, 1)
            .map(drop)
            .map_err(|error| error.to_string());
    };
    let parts = asked.parts()?;
    let index = parts
        .iter()
        .position(|one| one == part)
        .ok_or("the job's page is cut into no such part")?;
    let joined = page::capture_part(
        &chrome,
        &tools,
        &args.project,
        laid,
        &request,
        &parts,
        index,
    )
    .map_err(|error| error.to_string())?;
    if joined.is_none() && index + 1 == parts.len() {
        return Err(
            "the page's earlier parts were gone before its last part could join them; \
                    it is captured again when next asked"
                .to_owned(),
        );
    }
    Ok(())
}

/// `frames` as `--frames` takes it: `START..END`.
pub fn frames_arg(frames: &Range<u64>) -> String {
    format!("{}..{}", frames.start, frames.end)
}

/// `--frames`' value: `START..END`, with `START` before `END`.
fn frames(text: &str) -> Result<Range<u64>, String> {
    let (start, end) = text.split_once("..").ok_or("START..END")?;
    let parse = |part: &str| {
        part.parse::<u64>()
            .map_err(|_| "START..END, whole frames".to_owned())
    };
    let (start, end) = (parse(start)?, parse(end)?);
    if start >= end {
        return Err("START comes before END".to_owned());
    }
    Ok(start..end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_part_travels_as_start_dot_dot_end() {
        assert_eq!(frames(&frames_arg(&(1800..2000))), Ok(1800..2000));
        for wrong in ["1800", "2000..1800", "5..5", "a..b", "-1..3", "1..2..3"] {
            assert!(frames(wrong).is_err(), "{wrong}");
        }
    }
}
