//! `scorsese stock` — free stock footage and photos from Pixabay (#900).
//!
//! The same two steps as the `stock_search` and `stock_import` tools, over the
//! same library code: a search writes a contact sheet of its results'
//! previews to look at, and an import brings the chosen ids in as ordinary
//! assets. Nothing here spends money.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use scorsese_core::Project;
use scorsese_providers::credentials::{Provider, resolve};
use scorsese_providers::stock::{
    self, Choice, LICENCE, Medium, Orientation, PixabayLibrary, Query, cache_dir, footage, previews,
};
use scorsese_render::contact::{self, Look};
use scorsese_render::{Ffprobe, Tools, frames};

use crate::cli::StockAction;

/// Runs one `stock` action on the project in `dir`.
pub(crate) fn run(dir: &Path, action: StockAction) -> Result<()> {
    // Opened first because the cache and the assets live inside it.
    let mut project =
        Project::load(dir).with_context(|| format!("opening the project in {}", dir.display()))?;
    let key = resolve(Provider::Pixabay)?;
    let library = PixabayLibrary::new(&key.secret);
    let cache = cache_dir(dir);
    match action {
        StockAction::Search {
            look: Some(id),
            out,
            ..
        } => looked(
            &cache,
            &library,
            id,
            &out.unwrap_or_else(|| cache.join("look.png")),
        ),
        StockAction::Search {
            words,
            image,
            orientation,
            style,
            min_seconds,
            page,
            unsafe_results,
            out,
            ..
        } => {
            let query = Query {
                medium: if image { Medium::Image } else { Medium::Video },
                words: words.join(" "),
                style,
                orientation: orientation.as_deref().map(oriented).transpose()?,
                min_seconds,
                safe: !unsafe_results,
            };
            searched(
                &cache,
                &library,
                &query,
                page,
                &out.unwrap_or_else(|| cache.join("sheet.png")),
            )
        }
        StockAction::Import {
            ids,
            image,
            resolution,
        } => {
            let medium = if image { Medium::Image } else { Medium::Video };
            let choices: Vec<Choice> = ids.iter().map(|&id| Choice { medium, id }).collect();
            let probe = Ffprobe::discover()?;
            let frame = (resolution.width(), resolution.height());
            let answers =
                stock::import(&mut project, dir, &cache, &library, &choices, frame, &probe);
            let mut failed = 0;
            for (id, answer) in ids.iter().zip(&answers) {
                match answer {
                    Ok(one) => println!(
                        "{} — {} ({}x{}), by {} — {}{}",
                        one.imported.id,
                        one.rendition.name,
                        one.rendition.width,
                        one.rendition.height,
                        one.candidate.author,
                        one.candidate.page_url,
                        if one.fills {
                            ""
                        } else {
                            " — smaller than the frame"
                        }
                    ),
                    Err(error) => {
                        failed += 1;
                        eprintln!("{id} — failed: {error}");
                    }
                }
            }
            if answers
                .iter()
                .any(|answer| answer.as_ref().is_ok_and(|one| !one.imported.reused))
            {
                project.save(dir).context("saving the project")?;
            }
            if failed == answers.len() {
                bail!("nothing was imported");
            }
            println!("From Pixabay. {LICENCE}");
            Ok(())
        }
    }
}

/// `horizontal` or `vertical`.
fn oriented(word: &str) -> Result<Orientation> {
    match word {
        "horizontal" => Ok(Orientation::Horizontal),
        "vertical" => Ok(Orientation::Vertical),
        other => bail!("--orientation is horizontal or vertical, not `{other}`"),
    }
}

/// Prints a page of results and writes their sheet to `out`.
fn searched(
    cache: &Path,
    library: &PixabayLibrary,
    query: &Query,
    page: u32,
    out: &Path,
) -> Result<()> {
    let found = stock::search(cache, library, query, page)?;
    for (index, one) in found.candidates.iter().enumerate() {
        println!(
            "{}. {}\n   by {} — {}",
            index + 1,
            one.says(),
            one.author,
            one.page_url
        );
    }
    println!("\n{}", found.summary());
    if found.candidates.is_empty() {
        return Ok(());
    }
    let pictures: Vec<(PathBuf, String)> = previews(cache, library, &found.candidates)
        .into_iter()
        .zip(found.candidates.iter().enumerate())
        .filter_map(|(path, (index, one))| Some((path?, one.label(index))))
        .collect();
    let tools = Tools::discover()?;
    let vertical = query.orientation == Some(Orientation::Vertical);
    let sheet = contact::pictures(&tools, &pictures, vertical)?;
    frames::write_png(&tools, out, &sheet).with_context(|| format!("writing {}", out.display()))?;
    println!("Previews: {}", out.display());
    println!("Import with `scorsese stock import <id>`. {LICENCE}");
    Ok(())
}

/// Writes five frames across video `id` to `out`.
fn looked(cache: &Path, library: &PixabayLibrary, id: u64, out: &Path) -> Result<()> {
    let (candidate, file) = footage(cache, library, id)?;
    let tools = Tools::discover()?;
    let seconds = f64::from(candidate.seconds.unwrap_or_default());
    let range = Look {
        to_seconds: (seconds > 0.0).then_some(seconds),
        ..Look::default()
    };
    let sheet = contact::sheet(&tools, &file, &range)?;
    frames::write_png(&tools, out, &sheet.image)
        .with_context(|| format!("writing {}", out.display()))?;
    println!("{}\nFrames: {}", candidate.says(), out.display());
    Ok(())
}
