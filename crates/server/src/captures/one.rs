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
    let follow = &args.follow;
    page::capture_frames(&chrome, &tools, &args.project, follow, &request, frames, 1)
        .map(drop)
        .map_err(|error| error.to_string())
}
