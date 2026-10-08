//! `scorsese stock` — free stock footage and photos from Pixabay (#900), and
//! Lottie animations from LottieFiles (#903).
//!
//! The same two steps as the `stock_search` and `stock_import` tools, over the
//! same library code: a search writes a contact sheet of its results'
//! previews to look at, and an import brings the chosen ids in — as ordinary
//! assets, or a Lottie as a file under `pages/` for a page to play. Nothing
//! here spends money.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use scorsese_core::Project;
use scorsese_providers::stock::{
    self, Choice, Library, Medium, Orientation, Query, cache_dir, footage, licence, previews,
};
use scorsese_render::contact::{self, Look};
use scorsese_render::{Ffprobe, Tools, frames};

use crate::cli::StockAction;

/// Runs one `stock` action on the project in `dir`.
pub(crate) fn run(dir: &Path, action: StockAction) -> Result<()> {
    // Opened first because the cache and the assets live inside it.
    let mut project =
        Project::load(dir).with_context(|| format!("opening the project in {}", dir.display()))?;
    let cache = cache_dir(dir);
    match action {
        StockAction::Search {
            look: Some(id),
            lottie,
            out,
            ..
        } => {
            let medium = if lottie {
                Medium::Lottie
            } else {
                Medium::Video
            };
            looked(
                &cache,
                &*stock::library(medium)?,
                medium,
                id,
                &out.unwrap_or_else(|| cache.join("look.png")),
            )
        }
        StockAction::Search {
            words,
            image,
            lottie,
            orientation,
            style,
            min_seconds,
            page,
            unsafe_results,
            out,
            ..
        } => {
            let query = Query {
                medium: medium(image, lottie),
                words: words.join(" "),
                style,
                orientation: orientation.as_deref().map(oriented).transpose()?,
                min_seconds,
                safe: !unsafe_results,
            };
            searched(
                &cache,
                &*stock::library(query.medium)?,
                &query,
                page,
                &out.unwrap_or_else(|| cache.join("sheet.png")),
            )
        }
        StockAction::Import {
            ids, lottie: true, ..
        } => animations(dir, &cache, &ids),
        StockAction::Import {
            ids,
            image,
            resolution,
            ..
        } => {
            let medium = medium(image, false);
            let library = stock::library(medium)?;
            let choices: Vec<Choice> = ids.iter().map(|&id| Choice { medium, id }).collect();
            let probe = Ffprobe::discover()?;
            let frame = (resolution.width(), resolution.height());
            let answers = stock::import(
                &mut project,
                dir,
                &cache,
                &*library,
                &choices,
                frame,
                &probe,
            );
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
            println!("From Pixabay. {}", licence(medium));
            Ok(())
        }
    }
}

/// What `--image` and `--lottie` ask for: footage unless one is given.
const fn medium(image: bool, lottie: bool) -> Medium {
    match (image, lottie) {
        (_, true) => Medium::Lottie,
        (true, false) => Medium::Image,
        (false, false) => Medium::Video,
    }
}

/// Imports animations `ids` into `pages/` and says how a page plays them.
fn animations(dir: &Path, cache: &Path, ids: &[u64]) -> Result<()> {
    let library = stock::library(Medium::Lottie)?;
    let answers = stock::keep(dir, cache, &*library, ids);
    let mut failed = 0;
    for (id, answer) in ids.iter().zip(&answers) {
        match answer {
            Ok(one) => println!(
                "{} — \"{}\" by {}, {} frames at {} fps ({:.2}s){}",
                one.path,
                one.candidate.title,
                one.candidate.author,
                one.frames,
                one.fps,
                one.seconds(),
                if one.reused { ", already there" } else { "" }
            ),
            Err(error) => {
                failed += 1;
                eprintln!("{id} — failed: {error}");
            }
        }
    }
    if failed == answers.len() {
        bail!("nothing was imported");
    }
    println!(
        "Play it from an html page with lottie-web, driven from the page's clock: docs/pages.md, \
         A Lottie animation. From LottieFiles. {}",
        licence(Medium::Lottie)
    );
    Ok(())
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
    library: &dyn Library,
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
    let flag = match query.medium {
        Medium::Video => "",
        Medium::Image => " --image",
        Medium::Lottie => " --lottie",
    };
    println!(
        "Import with `scorsese stock import <id>{flag}`. {}",
        licence(query.medium)
    );
    Ok(())
}

/// Writes five frames across video or animation `id` to `out`.
fn looked(cache: &Path, library: &dyn Library, medium: Medium, id: u64, out: &Path) -> Result<()> {
    let (candidate, file) = footage(cache, library, medium, id)?;
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
