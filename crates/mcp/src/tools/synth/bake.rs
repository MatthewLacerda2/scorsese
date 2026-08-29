//! `synth_bake`, whole and partial.
//!
//! Its own file because it is two tools wearing one name, and the difference
//! between them is the thing a client most needs to be told. A plain bake is
//! the project's own audio: cached, addressed by the recipe and the
//! synthesiser, and what a clip will actually play. A bake with `beats`,
//! `seconds` or `only` on it is a **question** — how does this stretch sit,
//! what is that instrument doing — and its answer is deliberately not kept.
//!
//! So every argument here says it is not cached, and the reply says so again.
//! The failure this guards against is not a wasted render; it is somebody
//! reaching for a fragment as though it were the bake.

use std::path::Path;

use scorsese_core::AssetId;
use scorsese_providers::synth::{self, Baked, Excerpt, Partial, Span, Window};
use serde_json::Value;

use super::super::inspect::load;
use super::super::{Costs, Reply, Tool, project_dir, project_property};

/// Render the recipes that are not already baked — or a stretch of one.
pub(in crate::tools) struct Bake;

impl Tool for Bake {
    fn name(&self) -> &'static str {
        "synth_bake"
    }

    fn description(&self) -> &'static str {
        "Render every synth_audio recipe whose sound is not already on disk, \
         into generated/. Safe to call repeatedly and free every time: a recipe \
         that has not changed is a cache hit that renders nothing, and one that \
         has changed is redone without anyone having to mark it stale. A bake \
         is named for the recipe and for the synthesiser that rendered it, so \
         an upgrade that changes how a recipe sounds is redone here too. Says how \
         each one came out: level, spectral balance and stereo width for the \
         whole file, then \
         a row per section of the arrangement, then a row per track of a song \
         saying which instrument is taking up the room. A signal, never a gate \
         — nothing about a level can fail a bake. \
         Give it `beats`, `seconds` or `only` and it renders LESS of one \
         recipe instead — a stretch of the piece, or a few of its tracks — \
         which is the cheap way to read a mix back while tuning it. What comes \
         back over a window is exactly what the whole bake would have there. \
         That output is NOT cached and is NOT the asset's audio: it goes to \
         cache/, the project is left untouched, and a full synth_bake is still \
         what makes the file a clip plays."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "asset": {
                    "type": "string",
                    "description": "Bake only this asset. Omit and every synth_audio \
                                    asset is considered. Required when asking for a \
                                    window or a solo, which are questions about one \
                                    piece of music."
                },
                "beats": {
                    "type": "string",
                    "description": "Render only these beats of the piece: `0:32`, \
                                    `16:`, `:32` — end-exclusive, and counted along \
                                    what is rendered, so under a `loop` fit they are \
                                    not the written arrangement's beats. Beats and \
                                    not bars: a song has no time signature, so eight \
                                    bars of four is `0:32`. NOT cached — the result \
                                    lands in cache/ and the asset keeps pointing at \
                                    its own full bake, because a fragment stored \
                                    under the whole recipe's address would leave the \
                                    project holding audio its recipe does not \
                                    describe."
                },
                "seconds": {
                    "type": "string",
                    "description": "The same window said in seconds of the rendered \
                                    piece: `0:12`, `8:`, `:12`. Give this or \
                                    `beats`, never both. NOT cached, for the reason \
                                    `beats` gives."
                },
                "only": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Render only these tracks, by the names the song's \
                                    notes use. The song's own fx and the master \
                                    limiter still run, so this is the mix with fewer \
                                    parts in it rather than a bare instrument, and a \
                                    track something is sidechained from is still \
                                    played so the duck is the one the mix has. This \
                                    is for a person to listen to — a report cannot \
                                    tell you the pad is warbling. NOT cached, for \
                                    the reason `beats` gives."
                },
                "out": {
                    "type": "string",
                    "description": "Where to write a partial bake. Omit and it lands \
                                    in cache/synth/<asset>.wav, which the next \
                                    partial bake of the same recipe overwrites."
                }
            },
            "required": ["project"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let mut project = load(&dir)?;
        let asset = arguments.get("asset").and_then(Value::as_str);

        if let Some(excerpt) = excerpt(arguments)? {
            let Some(id) = asset else {
                return Err(
                    "a window or a solo is a question about one recipe — name the `asset`"
                        .to_owned(),
                );
            };
            let id = AssetId::new(id);
            let out = arguments.get("out").and_then(Value::as_str).map(Path::new);
            let partial = synth::bake_partial(&project, &dir, &id, &excerpt, out)
                .map_err(|error| format!("{error}"))?;
            return Ok(said_partial(&id, &excerpt, &partial).into());
        }

        let baked = match asset {
            Some(id) => {
                let id = AssetId::new(id);
                let one = synth::bake_asset(&mut project, &dir, &id)
                    .map_err(|error| format!("{error}"))?;
                vec![(id, one)]
            }
            None => synth::bake_pending(&mut project, &dir).map_err(|error| format!("{error}"))?,
        };
        if baked.is_empty() {
            return Ok("no synth_audio assets — synth_new starts one".into());
        }
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;

        let fresh = baked.iter().filter(|(_, it)| it.is_fresh()).count();
        let lines: Vec<String> = baked.iter().map(|(id, it)| said(id, it)).collect();
        Ok(format!(
            "{}\n{fresh} rendered, {} cached, $0.00",
            lines.join("\n"),
            baked.len() - fresh
        )
        .into())
    }
}

/// What less of the recipe was asked for, or `None` for the ordinary bake.
///
/// Both units at once is refused rather than one silently winning: a window
/// has one clock, and a client that gave two does not know which it got.
fn excerpt(arguments: &Value) -> Result<Option<Excerpt>, String> {
    let span = |name: &str| -> Result<Option<Span>, String> {
        match arguments.get(name).and_then(Value::as_str) {
            Some(text) => text
                .parse::<Span>()
                .map(Some)
                .map_err(|problem| format!("`{name}`: {problem}")),
            None => Ok(None),
        }
    };
    let (beats, seconds) = (span("beats")?, span("seconds")?);
    if beats.is_some() && seconds.is_some() {
        return Err("give `beats` or `seconds`, not both — a window has one clock".to_owned());
    }
    let only: Vec<String> = arguments
        .get("only")
        .and_then(Value::as_array)
        .map(|names| {
            names
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let window = beats
        .map(Window::beats)
        .or_else(|| seconds.map(Window::seconds));
    if window.is_none() && only.is_empty() {
        return Ok(None);
    }
    Ok(Some(Excerpt { window, only }))
}

/// One asset's line, and the tables under it.
///
/// An assistant that just rewrote a score should be able to read back how it
/// came out without asking a human to listen — and the whole table, not one
/// number, because the question it is usually answering is *where* in the
/// piece the change landed. The per-track rows are the other half of the same
/// question: a client that cannot hear can be told a mix is muddy, and can do
/// nothing with that until it is told which of five instruments is the mud.
fn said(id: &AssetId, outcome: &Baked) -> String {
    match outcome {
        Baked::Rendered {
            path,
            bytes,
            profile,
            tracks,
        } => {
            let head = format!(
                "{id} — baked, {} KB, {path}, {}",
                bytes / 1024,
                scorsese_render::say::summary(profile)
            );
            rows(
                head,
                &scorsese_render::say::sections(profile),
                &scorsese_render::say::layers(tracks),
            )
        }
        Baked::Cached { path } => format!("{id} — already baked, {path}"),
    }
}

/// The same report for a partial bake, ending in the sentence that keeps this
/// file from being mistaken for the asset's own.
fn said_partial(id: &AssetId, excerpt: &Excerpt, partial: &Partial) -> String {
    // The excerpt is in the headline because a level is a different finding
    // over eight bars than over the whole piece, and this line is the only
    // place a client is told which one it is reading.
    let head = format!(
        "{id} — part of it ({excerpt}), {} KB, {}, {}",
        partial.bytes / 1024,
        partial.shown,
        scorsese_render::say::summary(&partial.profile)
    );
    let mut said = rows(
        head,
        &scorsese_render::say::sections(&partial.profile),
        &scorsese_render::say::layers(&partial.tracks),
    );
    said.push_str(&format!(
        "\nnot cached, and not this asset's audio — synth_bake {id} with no window \
         is what makes that."
    ));
    said
}

/// A headline with the section and track tables indented under it.
fn rows(mut said: String, sections: &[String], layers: &[String]) -> String {
    for row in sections.iter().chain(layers) {
        said.push_str(&format!("\n  {row}"));
    }
    said
}
