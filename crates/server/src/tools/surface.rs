//! Which tools the web offers, and how each of the registry's is served.
//!
//! Every tool in `scorsese-mcp`'s registry gets exactly one answer from
//! [`serve`], and the test below fails on one that gets none — so a tool added
//! to the stdio server is a decision about the web the day it is added, not a
//! capability that quietly appears here or quietly never does.

use scorsese_mcp::{Tool, protocol};
use serde_json::{Value, json};

use super::own::Own;
use super::registered;

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
    /// By the server's own tool of the same name ([`Own`]).
    Replaced,
    /// Not on the web yet, for this reason.
    Withheld(&'static str),
}

/// Why MIDI in and out is not served.
const MIDI: &str = "a .mid file is neither media the library holds nor text a project keeps, \
                    so there is nothing to import from or to hand an export back as (#678)";

/// How the registry tool called `name` is served, or `None` for one nobody
/// has decided about — which the test below refuses.
pub(super) fn serve(name: &str) -> Option<Serve> {
    Some(match name {
        "project_read" | "project_describe" | "project_check" | "project_assets"
        | "project_probe" | "project_write" | "track_new" | "text_new" | "color_new"
        | "shape_new" | "icon_new" | "asset_set" | "sequence" | "place_clip" | "trim_clip"
        | "clip_set" | "clip_follow" | "clip_move" | "clip_remove" | "clip_group"
        | "clip_ungroup" | "dissolve" | "duck_music" | "set_volume" | "scale_pacing"
        | "rebrief" | "icons" | "voices" | "script_read" | "script_write" | "synth_new"
        | "synth_read" | "synth_write" | "synth_set" | "synth_check" | "synth_survey" => {
            Serve::Stored
        }
        "look" | "hear" => Serve::Confined(&["file"]),
        "audio_level" => Serve::Confined(&["file", "against"]),
        "still" | "synth_bake" => Serve::Without(&["out"]),
        "project_new" | "import" | "render" | "generate" | "voice_design" => Serve::Replaced,
        "synth_import" | "synth_export" => Serve::Withheld(MIDI),
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
        Serve::Replaced | Serve::Withheld(_) => None,
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
            Some(Serve::Withheld(_)) | None => {}
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
            Serve::Without(fields) => {
                for field in fields {
                    properties.remove(*field);
                }
            }
            _ => {}
        }
    }
    if let (Serve::Without(fields), Some(required)) = (serve, schema["required"].as_array_mut()) {
        required.retain(|name| !fields.iter().any(|field| name == field));
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
                    Some(Serve::Stored | Serve::Confined(_) | Serve::Without(_))
                )
            })
            .count()
    }

    #[test]
    fn a_withheld_or_replaced_tool_cannot_be_called_as_the_registry_has_it() {
        assert!(find("synth_export").is_none());
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
}
