//! `scorsese synth`

use std::path::Path;

use anyhow::{Context, Result, bail};
use scorsese_core::{AssetId, Project};
use scorsese_providers::synth::{self, Baked, Starter};
use scorsese_render::say;

/// Writes a starter recipe and the asset that points at it.
pub fn new(project_dir: &Path, name: &str, starter: Starter) -> Result<()> {
    let mut project = open(project_dir)?;
    let id = synth::create(&mut project, project_dir, name, starter)
        .with_context(|| format!("creating the recipe for `{name}`"))?;
    project.save(project_dir).context("saving the project")?;

    let asset = project.asset(&id).expect("the asset just created");
    let recipe = asset.recipe.as_ref().expect("a synth asset has a recipe");
    println!("{id} — synth_audio, sketch");
    println!("  {recipe}");
    println!("  edit it, then `scorsese synth bake` to hear it");
    Ok(())
}

/// Realises the synth assets whose recipes are not already baked.
///
/// With no id this covers every one of them, and it is deliberately safe to
/// re-run: an asset whose recipe has not changed is a cache hit that renders
/// nothing.
pub fn bake(project_dir: &Path, only: Option<&str>) -> Result<()> {
    let mut project = open(project_dir)?;
    let baked = match only {
        Some(id) => {
            let id = AssetId::new(id);
            let one = synth::bake_asset(&mut project, project_dir, &id)
                .with_context(|| format!("baking `{id}`"))?;
            vec![(id, one)]
        }
        None => synth::bake_pending(&mut project, project_dir).context("baking")?,
    };

    if baked.is_empty() {
        println!("no synth_audio assets — `scorsese synth new <name>` starts one");
        return Ok(());
    }
    project.save(project_dir).context("saving the project")?;
    report(&baked);
    Ok(())
}

/// Parses a recipe and says what it is, without rendering it.
pub fn check(recipe: &Path) -> Result<()> {
    let json =
        std::fs::read_to_string(recipe).with_context(|| format!("reading {}", recipe.display()))?;
    match synth::check(&json) {
        Ok(parsed) => {
            println!(
                "{}: a {} recipe, and it parses",
                recipe.display(),
                parsed.kind()
            );
            Ok(())
        }
        // Reported as a plain failure rather than an `anyhow` chain, because
        // serde's message already carries the line and column, which is the
        // whole of what a caller repairing the file needs.
        Err(problem) => bail!("{}: {problem}", recipe.display()),
    }
}

/// Says what the project's song recipes are made of, without baking any of
/// them.
///
/// Prints **nothing at all** for a project of fewer than two songs, and that is
/// the whole of what it does then: a survey is a report across a set, and a set
/// of one has no row a summary of it would not already be.
pub fn survey(project_dir: &Path) -> Result<()> {
    let project = open(project_dir)?;
    let surveyed = synth::survey(&project, project_dir).context("reading the recipes")?;
    for line in say::survey(&surveyed) {
        println!("{line}");
    }
    Ok(())
}

/// What each bake did, and one line saying how much of it was free.
fn report(baked: &[(AssetId, Baked)]) {
    for (id, outcome) in baked {
        match outcome {
            Baked::Rendered {
                path,
                bytes,
                profile,
                tracks,
            } => {
                println!("{id} — baked, {} KB", bytes / 1024);
                println!("  {path}");
                // A recipe's level is a property of the recipe, so saying it
                // here is saying it before the sound is ever mixed. The rows
                // under it are the arrangement's own sections, so a quiet
                // second chorus is named rather than looked up.
                println!("  {}", say::summary(profile));
                for row in say::sections(profile) {
                    println!("    {row}");
                }
                // Then the same figures split the other way — by instrument
                // rather than by time — because "the mix is muddy" and "the sub
                // is what is muddying it" are one finding and one guess.
                for row in say::layers(tracks) {
                    println!("    {row}");
                }
            }
            Baked::Cached { path } => {
                println!("{id} — already baked");
                println!("  {path}");
            }
        }
    }
    let fresh = baked.iter().filter(|(_, it)| it.is_fresh()).count();
    println!("{fresh} rendered, {} cached, $0.00", baked.len() - fresh);
}

fn open(project_dir: &Path) -> Result<Project> {
    Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))
}
