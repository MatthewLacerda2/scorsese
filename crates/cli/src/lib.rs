//! # scorsese-cli — the headless binary
//!
//! Responsibility: the `scorsese` command-line surface — `new`, `import`,
//! `check`, `render`, `synth`, `generate`, `assets`, `diff`. This is how an
//! agent (or a CI
//! job) assembles and renders a video with no human and no screen: every
//! editing capability the GUI will ever have must be reachable from here
//! first.
//!
//! Boundary: this crate must NEVER touch a display — no window, no GPU
//! surface, no GUI toolkit. It is glue: argument parsing and orchestration
//! over `scorsese-core`, `scorsese-render`, and `scorsese-providers`. Logic
//! that another caller (the MCP server, the GUI) would want lives in those
//! crates, not here.

pub mod cli;
pub mod commands;

use anyhow::Result;
use clap::Parser;

use cli::{AssetsAction, Cli, Command, SynthAction};

/// Parses the command line and runs it.
pub fn run() -> Result<()> {
    dispatch(Cli::parse())
}

fn dispatch(cli: Cli) -> Result<()> {
    let directory = cli.project_dir();
    match cli.command {
        Command::New {
            directory,
            name,
            fps,
        } => commands::new::run(&directory, name, fps),
        Command::Check { verify } => commands::check::run(&directory, verify),
        Command::Import { file, kind } => {
            commands::import::run(&directory, &file, kind.map(Into::into))
        }
        Command::Render {
            out,
            resolution,
            fps,
            bitrate,
            sample_rate,
            audio_bitrate,
            range,
            container,
            video_codec,
            audio_codec,
            stills,
            at,
        } => commands::render::run(
            &directory,
            &out,
            commands::render::Options {
                resolution,
                fps,
                bitrate,
                sample_rate,
                audio_bitrate,
                range,
                container,
                video_codec,
                audio_codec,
                stills,
                at,
            },
        ),
        Command::Describe { fps, range } => commands::describe::run(&directory, fps, range),
        Command::Synth { action } => match action {
            Some(SynthAction::New { name, kind }) => {
                commands::synth::new(&directory, &name, kind.into())
            }
            Some(SynthAction::Check { recipe }) => commands::synth::check(&recipe),
            Some(SynthAction::Bake { asset }) => {
                commands::synth::bake(&directory, asset.as_deref())
            }
            None => commands::synth::bake(&directory, None),
        },
        Command::Assets {
            action: None,
            verify,
        } => commands::assets::list(&directory, verify),
        Command::Assets {
            action: Some(AssetsAction::Gc { delete }),
            ..
        } => commands::assets::gc(&directory, delete),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// clap's own consistency checks — a malformed command tree is a panic at
    /// startup rather than a compile error, so it is worth asserting.
    #[test]
    fn the_command_tree_is_well_formed() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
