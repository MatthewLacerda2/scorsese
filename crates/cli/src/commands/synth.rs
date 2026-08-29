//! `scorsese synth`

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use scorsese_core::{AssetId, Project};
use scorsese_providers::synth::{self, Baked, Excerpt, Partial, Span, Starter, Window};
use scorsese_render::say;

/// What `synth bake` was asked for beyond "everything that is not on disk".
///
/// Together they make an [`Excerpt`], and *whether any of them was given* is
/// what decides between the two things this command does: the ordinary cached
/// bake, and a partial one that is never cached.
#[derive(Debug, Default)]
pub(crate) struct Less {
    /// A window in beats of the rendered piece.
    pub(crate) beats: Option<Span>,
    /// The same window in seconds. Clap refuses both at once.
    pub(crate) seconds: Option<Span>,
    /// The tracks to hear, by name. Empty is all of them.
    pub(crate) only: Vec<String>,
    /// Where a partial bake goes. `None` is `cache/synth/<asset>.wav`.
    pub(crate) out: Option<PathBuf>,
}

impl Less {
    /// The excerpt this asks for, or `None` when it asks for the whole piece.
    fn excerpt(&self) -> Option<Excerpt> {
        let window = self
            .beats
            .map(Window::beats)
            .or_else(|| self.seconds.map(Window::seconds));
        if window.is_none() && self.only.is_empty() {
            return None;
        }
        Some(Excerpt {
            window,
            only: self.only.clone(),
        })
    }
}

/// Writes a starter recipe and the asset that points at it.
pub(crate) fn new(project_dir: &Path, name: &str, starter: Starter) -> Result<()> {
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
pub(crate) fn bake(project_dir: &Path, only: Option<&str>, less: &Less) -> Result<()> {
    if let Some(excerpt) = less.excerpt() {
        // An excerpt is a question about one piece of music: which eight bars,
        // which track. Applying it across every recipe in the project would be
        // asking it of documents that never heard it, so the asset is
        // required rather than guessed at.
        let Some(id) = only else {
            bail!(
                "a window or a solo is a question about one recipe — name the asset, \
                 as in `scorsese synth bake trilha --beats 0:32`"
            );
        };
        let id = AssetId::new(id);
        let partial = synth::bake_partial(
            &open(project_dir)?,
            project_dir,
            &id,
            &excerpt,
            less.out.as_deref(),
        )
        .with_context(|| format!("baking part of `{id}`"))?;
        report_partial(&id, &excerpt, &partial);
        return Ok(());
    }
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
pub(crate) fn check(recipe: &Path) -> Result<()> {
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
pub(crate) fn survey(project_dir: &Path) -> Result<()> {
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

/// What a partial bake did, and the one line that says it is not the file the
/// project will use.
fn report_partial(id: &AssetId, excerpt: &Excerpt, partial: &Partial) {
    // Named in the headline, because a level is a different finding over eight
    // bars than over the whole piece and this line is the only place a reader
    // is told which they are looking at.
    println!("{id} — part of it ({excerpt}), {} KB", partial.bytes / 1024);
    println!("  {}", partial.shown);
    println!("  {}", say::summary(&partial.profile));
    for row in say::sections(&partial.profile) {
        println!("    {row}");
    }
    for row in say::layers(&partial.tracks) {
        println!("    {row}");
    }
    // Said every time rather than once in the help, because the whole risk
    // this feature carries is somebody reaching for this file as the bake.
    println!("  not cached, and not the asset's bake — `scorsese synth bake {id}` makes that");
}

fn open(project_dir: &Path) -> Result<Project> {
    Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))
}
