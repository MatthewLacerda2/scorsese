//! Which tools the web offers, and how each of the registry's is served.
//!
//! Every tool in `scorsese-mcp`'s registry gets exactly one answer from
//! [`serve`], and the test below fails on one that gets none — so a tool added
//! to the stdio server is a decision about the web the day it is added, not a
//! capability that quietly appears here or quietly never does.

use scorsese_mcp::{Tool, protocol};
use scorsese_providers::synth;
use serde_json::{Value, json};

use super::own::Own;
use super::registered;
use crate::library::Kind;

#[cfg(test)]
mod page;

/// How a registry tool is served on the web.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Serve {
    /// On the stored project, exactly as it is.
    Stored,
    /// On the stored project, with these arguments held to paths inside it.
    ///
    /// Locally a file argument may name anything on the machine — footage
    /// nobody imported yet. On the server that machine is everybody's, so a
    /// path is refused unless it stays inside the laid-out project, where
    /// every file is one of the caller's own.
    Confined(&'static [&'static str]),
    /// On the stored project, with these arguments refused: what they would
    /// write lands in a folder that is gone the moment the tool answers.
    Without(&'static [&'static str]),
    /// On the stored project, with the file argument `field` given instead as
    /// `item`, the id of one of the caller's library files of `kind`.
    ///
    /// For a file nothing in a project could hold: a `.mid` is not media and
    /// not a kept recipe, so it lives in the library, and is linked into the
    /// folder for the length of the call (`carried`).
    FromLibrary {
        /// The registry's argument the library file's path is handed in as.
        field: &'static str,
        /// What the item has to be.
        kind: Kind,
    },
    /// On the stored project, with the arguments `without` refused, and every
    /// file the tool writes under `dir` kept in the caller's library as `kind`
    /// — where the folder that is gone the moment the tool answers would
    /// otherwise have taken it (`carried`).
    IntoLibrary {
        /// The arguments refused: where to write, which the web decides.
        without: &'static [&'static str],
        /// The folder, inside the project, the tool writes to by default.
        dir: &'static str,
        /// What each file kept there is.
        kind: Kind,
    },
    /// By the server's own tool of the same name ([`Own`]).
    Replaced,
}

/// The registry tools served on the stored project exactly as they are — the
/// ones that read and write the document, and look things up for it.
///
/// One name a line, and a list rather than a `match` arm joined by `|`: rustfmt
/// packs an arm several names to a line, so every new tool rewrapped the lines
/// around it and any two pull requests adding one conflicted (#691). A long
/// list with a trailing comma stays one element a line, so adding a tool is one
/// added line. `docs/web.md` lists these by generation (`page` below).
const STORED: &[&str] = &[
    "project_read",
    "project_describe",
    "project_check",
    "project_assets",
    "project_probe",
    "project_write",
    "track_new",
    "text_new",
    "color_new",
    "shape_new",
    "icon_new",
    "asset_set",
    "sequence",
    "asset_remove",
    "track_remove",
    "place_clip",
    "trim_clip",
    "clip_set",
    "clip_animate",
    "clip_follow",
    "clip_move",
    "clip_remove",
    "clip_group",
    "clip_ungroup",
    "dissolve",
    "duck_music",
    "set_volume",
    "scale_pacing",
    "rebrief",
    "icons",
    "voices",
];

/// The registry tools served as they are whose files — the script and the
/// recipes — are the project's own `project_files` rather than its document.
/// Served exactly like [`STORED`]; listed apart because `docs/web.md` says
/// where what they write is kept.
const PROJECT_FILES: &[&str] = &[
    "script_read",
    "script_write",
    "synth_new",
    "synth_kit",
    "synth_read",
    "synth_write",
    "synth_set",
    "synth_check",
    "synth_survey",
];

/// The registry tools the server serves its own tool of the same name in
/// place of ([`Own`]), one a line for the reason [`STORED`] gives.
const REPLACED: &[&str] = &[
    "project_new",
    "import",
    "render",
    "jobs",
    "job_cancel",
    "generate",
    "voice_design",
];

/// How the registry tool called `name` is served, or `None` for one nobody
/// has decided about — which the test below refuses.
pub(super) fn serve(name: &str) -> Option<Serve> {
    if STORED.contains(&name) || PROJECT_FILES.contains(&name) {
        return Some(Serve::Stored);
    }
    if REPLACED.contains(&name) {
        return Some(Serve::Replaced);
    }
    Some(match name {
        "look" | "hear" => Serve::Confined(&["file"]),
        "audio_level" => Serve::Confined(&["file", "against"]),
        "still" | "synth_bake" => Serve::Without(&["out"]),
        "synth_import" => Serve::FromLibrary {
            field: "path",
            kind: Kind::Midi,
        },
        "synth_export" => Serve::IntoLibrary {
            without: &["out"],
            dir: synth::MIDI_EXPORT_DIR,
            kind: Kind::Midi,
        },
        _ => return None,
    })
}

/// A tool the web runs.
pub(super) enum Entry {
    /// One of the registry's, run on a stored project.
    Shared(Box<dyn Tool>, Serve),
    /// One of the server's own.
    Own(Own),
}

/// The tool called `name`, if the web serves one.
pub(super) fn find(name: &str) -> Option<Entry> {
    if let Some(own) = Own::named(name) {
        return Some(Entry::Own(own));
    }
    let tool = registered(name)?;
    match serve(name)? {
        Serve::Replaced => None,
        serve => Some(Entry::Shared(tool, serve)),
    }
}

/// Every tool, in the registry's order — a client reads the list in order —
/// with each server tool where the registry tool it replaces stands.
pub(super) fn listing() -> Vec<Value> {
    let mut listed = Vec::new();
    for tool in scorsese_mcp::registry() {
        match serve(tool.name()) {
            Some(Serve::Replaced) => {
                listed.extend(Own::replacing(tool.name()).iter().map(|own| own.listing()));
            }
            None => {}
            Some(serve) => listed.push(shown(tool.as_ref(), serve)),
        }
    }
    listed.extend(Own::LAST.iter().map(|own| own.listing()));
    listed
}

/// The `project` property as every web tool spells it.
pub(super) fn project_property() -> Value {
    json!({
        "type": "integer",
        "description": "The id of the project to work on — one of yours, as project_list \
                        shows it."
    })
}

/// The library file a [`Serve::FromLibrary`] argument becomes, described for
/// the web.
fn library_item(kind: Kind) -> Value {
    json!({
        "type": "integer",
        "description": format!(
            "The {} file to read, by the id `library` lists — one of the files in your \
             library. A file reaches the library by uploading it in the web app.",
            kind.label()
        )
    })
}

/// A confined file argument, described for the web.
const CONFINED: &str = "The file, as a path inside the project — assets/… or generated/…, as \
                        project_read and project_assets show each asset's path. Only files in \
                        the project can be read here.";

/// A registry tool as the web lists it: its own name and description, with
/// `project` an id and its file arguments described as the web holds them.
fn shown(tool: &dyn Tool, serve: Serve) -> Value {
    let mut listed = protocol::listing(tool);
    let schema = &mut listed["inputSchema"];
    if let Some(properties) = schema["properties"].as_object_mut() {
        properties.insert("project".to_owned(), project_property());
        match serve {
            Serve::Confined(fields) => {
                for field in fields {
                    if let Some(property) = properties.get_mut(*field) {
                        property["description"] = json!(CONFINED);
                    }
                }
            }
            Serve::Without(fields)
            | Serve::IntoLibrary {
                without: fields, ..
            } => {
                for field in fields {
                    properties.remove(*field);
                }
            }
            Serve::FromLibrary { field, kind } => {
                properties.remove(field);
                properties.insert("item".to_owned(), library_item(kind));
            }
            _ => {}
        }
    }
    if let Some(required) = schema["required"].as_array_mut() {
        match serve {
            Serve::Without(fields)
            | Serve::IntoLibrary {
                without: fields, ..
            } => required.retain(|name| !fields.iter().any(|field| name == field)),
            Serve::FromLibrary { field, .. } => {
                for name in required.iter_mut().filter(|name| *name == field) {
                    *name = json!("item");
                }
            }
            _ => {}
        }
    }
    listed
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every registry tool has been decided about, and every server tool
    /// either stands alone or replaces one the registry says is replaced.
    #[test]
    fn every_registry_tool_has_one_answer_and_no_name_is_claimed_twice() {
        for tool in scorsese_mcp::registry() {
            assert!(serve(tool.name()).is_some(), "{} is undecided", tool.name());
        }
        for own in Own::ALL {
            if let Some(serve) = serve(own.name()) {
                assert_eq!(
                    serve,
                    Serve::Replaced,
                    "{} shadows a served tool",
                    own.name()
                );
            }
        }
        let names: Vec<String> = listing()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap_or_default().to_owned())
            .collect();
        let mut unique = names.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(
            unique.len(),
            names.len(),
            "a tool is listed twice: {names:?}"
        );
        assert_eq!(names.len(), Own::ALL.len() + served_from_registry());
    }

    fn served_from_registry() -> usize {
        scorsese_mcp::registry()
            .iter()
            .filter(|tool| {
                matches!(
                    serve(tool.name()),
                    Some(
                        Serve::Stored
                            | Serve::Confined(_)
                            | Serve::Without(_)
                            | Serve::FromLibrary { .. }
                            | Serve::IntoLibrary { .. }
                    )
                )
            })
            .count()
    }

    #[test]
    fn a_replaced_tool_cannot_be_called_as_the_registry_has_it() {
        assert!(matches!(
            find("synth_export"),
            Some(Entry::Shared(_, Serve::IntoLibrary { .. }))
        ));
        assert!(matches!(
            find("synth_new"),
            Some(Entry::Shared(_, Serve::Stored))
        ));
        assert!(matches!(find("import"), Some(Entry::Own(_))));
        assert!(matches!(
            find("still"),
            Some(Entry::Shared(_, Serve::Without(_)))
        ));
        let still = listing()
            .into_iter()
            .find(|tool| tool["name"] == "still")
            .expect("still is served");
        assert!(still["inputSchema"]["properties"].get("out").is_none());
        assert_eq!(
            still["inputSchema"]["properties"]["project"]["type"],
            "integer"
        );
    }

    /// `synth_import` takes a library id where the registry takes a path, and
    /// `synth_export` has nowhere to be told to write (#678).
    #[test]
    fn midi_is_read_from_the_library_and_written_back_to_it() {
        let shown = |name: &str| {
            listing()
                .into_iter()
                .find(|tool| tool["name"] == name)
                .unwrap_or_else(|| panic!("{name} is served"))["inputSchema"]
                .clone()
        };
        let import = shown("synth_import");
        assert!(import["properties"].get("path").is_none());
        assert_eq!(import["properties"]["item"]["type"], "integer");
        assert!(
            import["required"]
                .as_array()
                .unwrap()
                .contains(&json!("item"))
        );
        assert!(
            !import["required"]
                .as_array()
                .unwrap()
                .contains(&json!("path"))
        );
        let export = shown("synth_export");
        assert!(export["properties"].get("out").is_none());
        assert!(export["properties"].get("asset").is_some());
    }
}
