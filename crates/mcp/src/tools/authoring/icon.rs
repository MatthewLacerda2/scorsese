//! A symbol from the set this build ships, named rather than imported.

use schemars::JsonSchema;
use scorsese_core::{Icon, Inline, authoring};
use serde::Deserialize;
use serde_json::Value;

use super::{COLOR, SIZE, STROKE_WIDTH, color, id_described, maybe, refused, save};
use crate::tools::args::{self, Name, ProjectDir, Required};
use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool};

/// Add an `icon` asset.
pub(crate) struct IconNew;

/// What `icon_new` takes.
#[derive(Deserialize, JsonSchema)]
struct Arguments {
    project: ProjectDir,
    /// Which symbol, by the catalogue's own name for it — lowercase and
    /// hyphenated, `clapperboard` or `circle-play`. The `icons` tool finds one
    /// from a word.
    name: Name,
    #[schemars(description = SIZE)]
    size: f64,
    #[schemars(description = COLOR)]
    color: Name,
    #[schemars(description = STROKE_WIDTH)]
    stroke_width: Option<f64>,
    #[schemars(description = id_described("the symbol's name"))]
    asset: Option<String>,
}

impl args::Arguments for Arguments {
    const REQUIRED: Required = &[
        ("name", "which symbol to draw"),
        ("size", "how big, as a fraction of the frame's height"),
        ("color", "the colour to draw it in"),
    ];
}

impl Tool for IconNew {
    fn name(&self) -> &'static str {
        "icon_new"
    }

    fn description(&self) -> &'static str {
        "Add an icon asset: one of the seventeen hundred symbols this build \
         ships, named rather than imported. Call `icons` first to find the name \
         — a name that is not in the catalogue is refused by project_check and \
         by the render, not here. A name is a few bytes, sharp at 4K, and \
         recoloured by editing one string, which is the whole reason not to \
         author a symbol as a PNG somewhere else. Size and colour are both \
         required: a symbol drawn at a size nobody chose, in a colour nobody \
         chose, is a shot that is wrong with nothing to say so. Validated before \
         it is written, and a refusal changes nothing."
    }

    fn costs(&self) -> Costs {
        Costs::Nothing
    }

    fn schema(&self) -> Value {
        args::schema::<Arguments>()
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let arguments: Arguments = args::parse(arguments)?;
        let dir = arguments.project.dir();
        let mut project = load(dir)?;
        let drawn_in = color(Some(arguments.color.as_str()), "color")?
            .ok_or("`color` is required: the colour to draw it in")?;
        let icon = Icon::new(arguments.name.as_str().to_owned(), arguments.size, drawn_in);
        let icon = match arguments.stroke_width {
            Some(width) => icon.weighing(width),
            None => icon,
        };
        let name = icon.name.clone();
        let id = authoring::add_asset(
            &mut project,
            maybe(arguments.asset.as_deref()).as_deref(),
            Inline::Icon(icon),
        )
        .map_err(refused)?;
        save(&project, dir)?;
        Ok(format!(
            "`{id}` — an icon asset drawing `{name}`. project_check says whether \
             that name is one this build ships."
        )
        .into())
    }
}
