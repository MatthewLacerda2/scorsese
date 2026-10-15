//! `project_assets`: what is in the media pool, one line an asset.

use schemars::JsonSchema;
use scorsese_core::{AssetKind, AssetStatus, HashCheck, Listed, Project, asset_status, listing};
use serde::Deserialize;
use serde_json::Value;

use super::load;
use crate::tools::args::{self, ProjectDir};
use crate::tools::{Costs, Reply, Tool};

/// What is in the media pool.
pub(in crate::tools) struct Assets;

/// What `project_assets` takes.
#[derive(Deserialize, JsonSchema)]
struct AssetsArguments {
    project: ProjectDir,
    /// Re-hash every file to catch media edited behind the project's back. Slow
    /// on a large pool; the default only checks that files are present.
    #[serde(default)]
    verify: bool,
}

impl args::Arguments for AssetsArguments {}

impl Tool for Assets {
    fn name(&self) -> &'static str {
        "project_assets"
    }

    fn description(&self) -> &'static str {
        "List the media pool: every asset, its kind, what state it is in, and \
         how many clips use it. One no clip shows says what names it instead \
         (a shot, a sequence, a page), which is what keeps it from asset_remove's \
         collection; a page or a synth_audio asset ends with the file it is written in \
         (its page, its recipe). An image sequence is one line with its count of \
         stills; only a still needing attention or also used by a clip is named, \
         indented under it. Says which generated assets are still sketches \
         nobody has realised, and which files the document points at and cannot \
         find."
    }

    fn writes(&self) -> bool {
        false
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<AssetsArguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: AssetsArguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let project = load(dir)?;
        let check = if arguments.verify {
            HashCheck::Verify
        } else {
            HashCheck::Skip
        };
        let rows = asset_status(&project, dir, check);
        if rows.is_empty() {
            return Ok("the pool is empty".into());
        }
        Ok(listing(&project, &rows)
            .iter()
            .flat_map(|one| listed(&project, one))
            .collect::<Vec<_>>()
            .join("\n")
            .into())
    }
}

/// One top-level asset, and for a sequence how many stills it plays (#684).
///
/// Collapsed the way the panels and `scorsese assets` show it: a 400-photo
/// timelapse costs a caller one line, not 401. The stills still named under it
/// are the ones worth acting on — one needing attention, or one a clip also
/// shows on its own. Every still id is in `project_read`'s document.
fn listed(project: &Project, listed: &Listed<'_>) -> Vec<String> {
    let mut lines = vec![line(project, listed.row)];
    if !listed.stills.is_empty() {
        lines[0].push_str(&format!("\t{} still(s) under it", listed.stills.len()));
    }
    lines.extend(listed.stills.iter().filter_map(|still| {
        if still.health.needs_attention() {
            Some(format!("  {}", line(project, still)))
        } else if still.clip_count > 0 {
            Some(format!("  {}\talso used on its own", line(project, still)))
        } else {
            None
        }
    }));
    lines
}

/// One asset: id, kind and health in the document's words, what uses it, and
/// for a page or a recipe-backed sound the file its content is written in.
fn line(project: &Project, row: &AssetStatus) -> String {
    let mut line = format!(
        "{}\t{}\t{}\t{} clip(s)",
        row.id, row.kind, row.health, row.clip_count
    );
    if let (0, Some(named)) = (row.clip_count, &row.named_by) {
        line.push_str(&format!(", {named}"));
    }
    let asset = project.asset(&row.id);
    let written = match row.kind {
        AssetKind::SynthAudio => asset.and_then(|asset| asset.recipe.as_ref()),
        AssetKind::Html => asset.and_then(|asset| asset.path.as_ref()),
        _ => None,
    };
    if let Some(path) = written {
        line.push_str(&format!("\t{path}"));
    }
    line
}
