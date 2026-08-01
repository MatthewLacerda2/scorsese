//! `scorsese assets` — list, and collect what nothing uses.

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::pool::{remove_assets, unused_assets};
use scorsese_core::{AssetHealth, AssetStatus, HashCheck, Project, asset_status};

/// Prints one line per asset: what it is, how many clips lean on it, and
/// whether the file behind it is still there and still the one that was
/// imported.
///
/// Re-hashing is opt-in because it is the only part of this that reads the
/// media, and a pool of large sources turns an instant listing into seconds.
pub(crate) fn list(project_dir: &Path, verify: bool) -> Result<()> {
    let project = open(project_dir)?;
    let check = if verify {
        HashCheck::Verify
    } else {
        HashCheck::Skip
    };
    let rows = asset_status(&project, project_dir, check);

    if rows.is_empty() {
        println!("No assets yet. `scorsese import <file>` adds one.");
        return Ok(());
    }
    for row in &rows {
        println!("{}", format_row(row));
    }

    let needing = rows
        .iter()
        .filter(|row| row.health.needs_attention())
        .count();
    let unused = rows.iter().filter(|row| row.clip_count == 0).count();
    println!(
        "\n{} assets, {unused} unused, {needing} needing attention",
        rows.len()
    );
    if !verify {
        println!("(hashes not checked — pass --verify to re-hash every file)");
    }
    Ok(())
}

/// Reports the assets no clip references, and only deletes them when asked
/// twice — the report is the default because unimported media is not
/// recoverable, and an agent running this unattended should have to mean it.
pub(crate) fn gc(project_dir: &Path, delete: bool) -> Result<()> {
    let mut project = open(project_dir)?;
    let unused = unused_assets(&project);

    if unused.is_empty() {
        println!("Every asset is used by at least one clip.");
        return Ok(());
    }
    for id in &unused {
        println!("unused: {id}");
    }

    if !delete {
        println!(
            "\n{} unused. Nothing deleted — pass --delete to remove them.",
            unused.len()
        );
        return Ok(());
    }

    let report = remove_assets(&mut project, project_dir, &unused).context("collecting assets")?;
    project.save(project_dir).context("saving the project")?;
    println!(
        "\nRemoved {} assets, deleted {} files, freed {}",
        report.removed.len(),
        report.files_deleted,
        human_bytes(report.bytes_freed)
    );
    Ok(())
}

fn open(project_dir: &Path) -> Result<Project> {
    Project::load(project_dir)
        .with_context(|| format!("opening the project in {}", project_dir.display()))
}

fn format_row(row: &AssetStatus) -> String {
    let uses = match row.clip_count {
        0 => "unused".to_owned(),
        1 => "1 clip".to_owned(),
        many => format!("{many} clips"),
    };
    format!(
        "{:<20} {:<16} {:<10} {}",
        row.id,
        format!("{:?}", row.kind),
        uses,
        health(&row.health)
    )
}

fn health(health: &AssetHealth) -> String {
    match health {
        AssetHealth::Ok => "ok".to_owned(),
        AssetHealth::Inline => "inline text".to_owned(),
        AssetHealth::Awaiting(state) => format!("awaiting generation ({state:?})"),
        AssetHealth::Missing => "FILE MISSING".to_owned(),
        AssetHealth::HashMismatch { .. } => "CHANGED SINCE IMPORT".to_owned(),
        AssetHealth::Unprobed => "not probed".to_owned(),
        AssetHealth::Unreadable(why) => format!("UNREADABLE: {why}"),
    }
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}
