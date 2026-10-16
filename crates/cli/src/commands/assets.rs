//! `scorsese assets` — list, and collect what nothing uses.

use std::path::Path;

use anyhow::{Context, Result};
use scorsese_core::pool::{Paid, named_by_pages, remove_assets, unused_assets};
use scorsese_core::{Asset, AssetId, AssetStatus, HashCheck, Listed, asset_status, listing};

/// Prints one line per asset: what it is, how many clips lean on it, and
/// whether the file behind it is still there and still the one that was
/// imported.
///
/// Re-hashing is opt-in because it is the only part of this that reads the
/// media, and a pool of large sources turns an instant listing into seconds.
pub(crate) fn list(project_dir: &Path, verify: bool) -> Result<()> {
    let project = super::open(project_dir)?;
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
    for listed in listing(&project, &rows) {
        print_listed(&listed);
    }

    let needing = rows
        .iter()
        .filter(|row| row.health.needs_attention())
        .count();
    // What `gc` lists: nothing shows it and nothing names it. A paid one
    // among them it keeps unless asked with `--include-paid`.
    let unused = rows
        .iter()
        .filter(|row| row.clip_count == 0 && row.named_by.is_none())
        .count();
    println!(
        "\n{} assets, {unused} unused, {needing} needing attention",
        rows.len()
    );
    if !verify {
        println!("(hashes not checked — pass --verify to re-hash every file)");
    }
    Ok(())
}

/// Reports the assets nothing uses, and only deletes them when asked twice —
/// the report is the default because unimported media is not recoverable, and
/// an agent running this unattended should have to mean it.
///
/// A picture a page's text names is kept and listed as kept: nothing in the
/// document records that use, so the reason has to be said. So is media paid
/// for from a provider, unless `include_paid` says the user asked for it to
/// go: what was bought is never deleted on a tidy-up's say-so.
pub(crate) fn gc(project_dir: &Path, delete: bool, include_paid: bool) -> Result<()> {
    let mut project = super::open(project_dir)?;
    let named = named_by_pages(&project, project_dir);
    let paid = |id: &AssetId| project.asset(id).is_some_and(Asset::is_paid);

    let mut kept = 0;
    let mut unused = Vec::new();
    for id in unused_assets(&project) {
        if let Some(by) = named.iter().find(|by| by.asset == id) {
            println!("kept: {id} — named by page {}", by.page);
            kept += 1;
        } else if paid(&id) && !include_paid {
            println!("kept: {id} — paid for; pass --include-paid to delete it");
            kept += 1;
        } else {
            unused.push(id);
        }
    }
    if unused.is_empty() {
        let none = if kept == 0 {
            "Every asset is in use."
        } else {
            "Nothing to collect."
        };
        println!("{none}");
        return Ok(());
    }
    for id in &unused {
        let note = if paid(id) { " (paid for)" } else { "" };
        println!("unused: {id}{note}");
    }

    if !delete {
        println!(
            "\n{} unused. Nothing deleted — pass --delete to remove them.",
            unused.len()
        );
        return Ok(());
    }

    // Every paid id still in the list was asked for by `--include-paid`.
    let report = remove_assets(&mut project, project_dir, &unused, Paid::Delete)
        .context("collecting assets")?;
    super::save(&project, project_dir)?;
    println!(
        "\nRemoved {} assets, deleted {} files, freed {}",
        report.removed.len(),
        report.files_deleted,
        human_bytes(report.bytes_freed)
    );
    Ok(())
}

/// A top-level asset, and for a sequence the count of its stills (#684).
///
/// Collapsed, as the panels show it: a 400-photo timelapse is one line, not
/// 401. The stills that *are* named under it are the ones worth a look — one
/// needing attention, or one a clip also shows on its own — so a missing
/// photo still surfaces here, where a broken pool is looked for.
fn print_listed(listed: &Listed<'_>) {
    let mut line = format_row(listed.row);
    if !listed.stills.is_empty() {
        line.push_str(&format!(" — {}", stills(listed.stills.len())));
    }
    println!("{line}");
    for still in &listed.stills {
        if still.health.needs_attention() {
            println!("  └ {}", format_row(still));
        } else if still.clip_count > 0 {
            println!("  └ {} (also used on its own)", format_row(still));
        }
    }
}

fn stills(count: usize) -> String {
    match count {
        1 => "1 still".to_owned(),
        many => format!("{many} stills"),
    }
}

fn format_row(row: &AssetStatus) -> String {
    let uses = match (row.clip_count, &row.named_by) {
        // Named by a shot, a sequence or a page: in use, and `gc` keeps it.
        (0, Some(named)) => named.to_string(),
        (0, None) => "unused".to_owned(),
        (1, _) => "1 clip".to_owned(),
        (many, _) => format!("{many} clips"),
    };
    format!(
        "{:<20} {:<16} {:<10} {}",
        row.id, row.kind, uses, row.health
    )
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
