//! `scorsese vectorize` — a picture traced into an SVG a page draws on.
//!
//! The command-line half of the `vectorize` MCP tool, over the same function
//! and answering in the same words.

use std::path::Path;

use anyhow::{Context, Result, bail};
use clap::Args;
use scorsese_core::Project;
use scorsese_render::Tools;
use scorsese_render::trace::{Detail, Tracing, vectorize};

/// What `scorsese vectorize` takes.
#[derive(Debug, Args)]
pub(crate) struct Options {
    /// The picture's asset id: an image, or a generated_image that has been
    /// generated.
    asset: String,
    /// What to call the drawing: the file is pages/<name>.svg and its group of
    /// marks has id <name>. Defaults to the asset id. Tracing again under the
    /// same name replaces the file.
    #[arg(long)]
    name: Option<String>,
    /// At most this many colours, the background included: 2 to 32. A flat
    /// illustration needs as many as it has; more only traces shading.
    #[arg(long, default_value_t = 8, value_parser = clap::value_parser!(u8).range(2..=32))]
    colours: u8,
    /// How much small detail survives: low (bold shapes), medium, or high
    /// (small marks kept, more paths).
    #[arg(long, default_value = "medium", value_parser = ["low", "medium", "high"])]
    detail: String,
    /// Keep the picture's background colour as a shape, rather than letting
    /// the page show around the drawing.
    #[arg(long)]
    keep_background: bool,
}

/// Traces the picture and says what was written.
pub(crate) fn run(project_dir: &Path, options: &Options) -> Result<()> {
    let Some(detail) = Detail::named(&options.detail) else {
        bail!("--detail is low, medium or high");
    };
    let tracing = Tracing {
        colours: usize::from(options.colours),
        detail,
        keep_background: options.keep_background,
    };
    let project = Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))?;
    let tools = Tools::discover()?;
    let name = options.name.as_deref().unwrap_or(&options.asset);
    let written = vectorize(&tools, &project, project_dir, &options.asset, name, tracing)?;
    println!("{}", written.summary());
    println!(
        "Draw it on with kit.draw from a page: `scorsese guide pages`, *A traced picture, drawn on*."
    );
    Ok(())
}
