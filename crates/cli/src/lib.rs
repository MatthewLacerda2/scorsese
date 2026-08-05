//! # scorsese-cli — the headless binary
//!
//! Responsibility: the `scorsese` command-line surface — `new`, `import`,
//! `check`, `render`, `still`, `synth`, `duck`, `voices`, `generate`,
//! `assets`, `diff`.
//! This is how an agent (or a CI
//! job) assembles and renders a video with no human and no screen: every
//! editing capability the GUI will ever have must be reachable from here
//! first.
//!
//! Boundary: this crate must NEVER touch a display — no window, no GPU
//! surface, no GUI toolkit. It is glue: argument parsing and orchestration
//! over `scorsese-core`, `scorsese-render`, and `scorsese-providers`. Logic
//! that another caller (the MCP server, the GUI) would want lives in those
//! crates, not here.
//!
//! ## What this publishes
//!
//! Almost nothing, which is the right shape for a binary crate: [`run`], which
//! is what `main` calls, plus the two things this crate's own tests reach for
//! from outside it. [`Cli`] is the command tree as a value, so the help-text
//! gate can walk every verb and flag without running any of them, and
//! [`mod@media`] is the health-to-severity table `check` prints, so the rule
//! deciding what fails a project can be asserted directly instead of being read
//! back out of stdout.
//!
//! Everything else is `pub(crate)`. Nothing in the workspace depends on this
//! crate — each `commands` module is one verb's worth of orchestration and
//! printing, and a second caller for any of it would be the sign that the logic
//! belongs in `scorsese-core` or `scorsese-render` instead.

mod cli;
mod commands;

use anyhow::Result;
use clap::Parser;

pub use cli::Cli;
pub use commands::check::media;

use cli::{AssetsAction, Command, SynthAction};
use scorsese_providers::voices::Filters;
use scorsese_render::contact::Look;

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
        Command::Settings => commands::settings::run(),
        Command::Generate {
            dry_run,
            yes,
            collect,
            wait,
        } => {
            if collect {
                commands::generate::sweep(&directory)
            } else {
                commands::generate::run(
                    &directory,
                    std::time::Duration::from_secs(wait),
                    dry_run,
                    yes,
                )
            }
        }
        Command::Voices {
            library,
            language,
            locale,
            gender,
            age,
            accent,
            page_size,
            refresh,
            check,
        } => commands::voices::run(
            &directory,
            &commands::voices::Options {
                library,
                filters: Filters {
                    language,
                    locale,
                    gender,
                    age,
                    accent,
                    page_size,
                },
                refresh,
                check,
            },
        ),
        Command::VoiceDesign {
            prompt,
            text,
            seed,
            guidance,
            dry_run,
            keep,
            name,
            list,
        } => commands::design::run(
            &directory,
            &commands::design::Options {
                prompt,
                passage: text,
                seed,
                guidance,
                dry_run,
                keep,
                name,
                list,
            },
        ),
        Command::Prices => commands::prices::run(),
        Command::Probe { all } => commands::probe::run(&directory, all),
        Command::Check { verify } => commands::check::run(&directory, verify),
        Command::Import { path, kind } => {
            commands::import::run(&directory, &path, kind.map(Into::into))
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
        Command::Still {
            at,
            out,
            resolution,
            grid,
        } => commands::still::run(&directory, &out, &at, resolution, grid),
        Command::Look {
            file,
            out,
            from,
            to,
            frames,
            grid,
        } => commands::look::run(
            &file,
            out,
            &Look {
                from_seconds: from,
                to_seconds: to,
                count: frames,
                grid,
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
            Some(SynthAction::Survey) => commands::synth::survey(&directory),
            None => commands::synth::bake(&directory, None),
        },
        Command::Dissolve { from, to, seconds } => {
            commands::dissolve::run(&directory, &from, &to, seconds)
        }
        Command::Duck {
            music,
            depth,
            attack,
            release,
            under,
        } => commands::duck::run(
            &directory,
            &music,
            &commands::duck::Options {
                depth,
                attack,
                release,
                under,
            },
        ),
        Command::Level { file, against } => commands::level::run(&file, against.as_deref()),
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
