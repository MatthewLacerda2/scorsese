//! The instrument library, as a client sees it.

use schemars::JsonSchema;
use scorsese_providers::synth::kit::{self, KIT};
use serde::Deserialize;
use serde_json::Value;

use crate::tools::args::{self, ProjectDir};
use crate::tools::{Costs, Reply, Tool};

/// List the library, or show one instrument's patch.
pub(in crate::tools) struct Kit;

/// What `synth_kit` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    // Taken and not used, as `icons` does: every tool names the project it
    // is called about, and the library is the same for all of them.
    #[expect(dead_code, reason = "taken for the uniform surface, not read")]
    project: ProjectDir,
    /// One instrument's name, as `kick` or `kit:kick`: its patch comes back as
    /// JSON, to read before using it or to edit a copy of. Without it, the
    /// whole library is listed, one line each.
    instrument: Option<String>,
}

impl args::Arguments for Arguments {}

impl Tool for Kit {
    fn name(&self) -> &'static str {
        "synth_kit"
    }

    fn description(&self) -> &'static str {
        "List the ready-made instruments a recipe can start from, or show one \
         instrument's patch. There is a drum machine's kick, snare, closed hat \
         and crash, a synth bass, a clav, a brass section, a pad and an \
         electric piano. A song track uses one by writing \"patch\": \"kit:kick\" and \
         passing the recipe to synth_write, which copies the patch into the \
         recipe: the song keeps its own copy, edits it freely, and never \
         changes sound when scorsese is upgraded. synth_new with an \
         `instrument` starts a one-shot of one. Costs nothing."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let Some(name) = arguments.instrument.as_deref() else {
            let lines: Vec<String> = KIT
                .iter()
                .map(|it| format!("{}{} — {}", kit::PREFIX, it.name, it.describes))
                .collect();
            return Ok(lines.join("\n").into());
        };
        let instrument = kit::lookup(name)
            .ok_or_else(|| format!("there is no `{name}` in the kit — synth_kit lists it"))?;
        Ok(instrument.json().trim_end().to_owned().into())
    }
}
