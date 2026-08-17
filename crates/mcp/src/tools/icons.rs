//! Which of seventeen hundred symbols is the one you meant.
//!
//! **The drawing work is wasted without this.** An assistant laying out a frame
//! knows it wants the film-camera one; it does not know whether upstream calls
//! that `film`, `video`, `camera` or `clapperboard`, and short of this tool the
//! only way to find out is to guess a name, have validation refuse it, and
//! guess again. The catalogue cannot be listed instead — seventeen hundred
//! names do not fit in a context window, which is the whole reason a search
//! exists rather than a listing.
//!
//! The likeliest wrong guess of all is a name the icon *used* to have, so those
//! are searched too and marked when they match. What comes back is still the
//! catalogue's own name either way, which is what keeps the answer something to
//! paste rather than something to interpret.
//!
//! It costs nothing and needs nothing: no bake, no ffmpeg, no network, no
//! provider. The table is compiled into this binary, so this is the standing
//! `synth_survey` has — a tool to call on a hunch rather than one to budget for.
//!
//! The lookup itself is [`scorsese_render::icon::search`], beside the
//! catalogue, which is what keeps this and `scorsese icons` one answer instead
//! of two.

use serde_json::Value;

use super::{Costs, Reply, Tool, project_dir, project_property};

/// Find an icon by a word.
pub(crate) struct Icons;

impl Tool for Icons {
    fn name(&self) -> &'static str {
        "icons"
    }

    fn description(&self) -> &'static str {
        "Find an icon by a word, and answer with names — each one a string to \
         write as an `icon` asset's `name`. This build ships the whole Lucide \
         set, seventeen hundred symbols, which is far too many to list: a word \
         is how you reach it. `query` is matched as a plain substring, \
         case-insensitive and never fuzzy, first against every icon's name, \
         then against the words it is filed under — which is what finds \
         `clapperboard` for \"film\", \"cinema\" or \"movie\", none of which is \
         in its name — and last against the names upstream retired, which is \
         what finds `lock-open` for \"unlock\". Exact names come first, then \
         names containing the word, then everything a tag matched, then \
         anything only a retired name matched, which is written \
         `lock-open (formerly unlock)`: the name to write is the one before \
         the brackets, always. The answer is capped and says both how \
         many matched and how many are shown, so a long list is never mistaken \
         for the whole of one; when it is capped, a narrower word is the way to \
         the rest. Nothing matching is an answer rather than an error — try \
         another word. Costs nothing and touches nothing: the catalogue is \
         compiled into this build, so it is the same set for every project and \
         needs no network, no bake and no file."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "project": project_property(),
                "query": {
                    "type": "string",
                    "description": "The word to look for — `camera`, `film`, \
                                    `warning`, `arrow`. Matched inside names, \
                                    tags and retired names alike, so a fragment \
                                    works (`clap` finds the clapperboard) and so \
                                    does the name an icon used to have (`unlock` \
                                    finds `lock-open`)."
                }
            },
            "required": ["project", "query"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        // Taken and not used, which is deliberate: every tool here names the
        // project it is called about, and the icon set is the same for all of
        // them. The description says so rather than leaving a client to wonder
        // whether a project can carry symbols of its own.
        project_dir(arguments)?;
        let query = arguments
            .get("query")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|query| !query.is_empty())
            .ok_or_else(|| {
                "`query` is required: a word to look for, like `camera` or `film`".to_owned()
            })?;
        Ok(scorsese_render::icon::search(query).to_string().into())
    }
}
