//! Making an image sequence from stills already in the pool, or retiming one.
//!
//! The half of a sequence's life that does not depend on where its stills came
//! from. A folder of frames on this machine comes in with `import` and
//! `sequence: true`; stills that arrived any other way — one at a time, or
//! from a hosted library — are turned into a sequence here, and every
//! sequence is retimed here, whichever way it arrived.

use scorsese_core::{AssetId, Frames, SequenceChange, change_sequence};
use serde_json::Value;

use crate::tools::inspect::load;
use crate::tools::{Costs, Reply, Tool, project_dir, project_property};

/// Make or change an `image_sequence` asset.
pub(crate) struct Sequence;

impl Tool for Sequence {
    fn name(&self) -> &'static str {
        "sequence"
    }

    fn description(&self) -> &'static str {
        "Make an image sequence from stills already in the pool, or change one: \
         which stills it plays in what order, how many frames each is held, and \
         whether it loops. An image sequence is a picture with a timeline of its \
         own — a timelapse, stop motion, a rendered frame directory, a flickering \
         sign or a few drawings cycling — and a clip shows it like footage: a \
         shorter clip shows less of it, a clip's speed plays it faster. An `asset` \
         id nothing answers to is made, from `stills`; one that is a sequence is \
         changed in exactly the fields named, and every field left out stays as \
         it is. Past its last still a sequence that loops starts again, and one \
         that does not holds its last still for as long as the clip lasts. Its \
         length is never written down — it is the stills times the hold. To bring \
         a whole folder of numbered frames in as one sequence, use import with \
         `sequence: true` instead. Nothing is written unless the whole document \
         still loads."
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
                    "description": "Id of the image_sequence asset to change, or the id \
                                    to make one under. A clip shows it by this id."
                },
                "stills": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "The image assets it plays, in order, by id — \
                                    replacing the whole list. An id may appear more \
                                    than once. Every one must be an imported image, \
                                    all of one format and one size. Required to make \
                                    a new sequence."
                },
                "hold": {
                    "type": "integer",
                    "minimum": 1,
                    "description": "How many timeline frames each still stays on \
                                    screen. 1 for a timelapse or a rendered frame \
                                    directory; 2 to 4 for drawn animation."
                },
                "loop": {
                    "type": "boolean",
                    "description": "true to start again from the first still when it \
                                    runs out; false to hold the last still instead."
                }
            },
            "required": ["project", "asset"]
        })
    }

    fn call(&self, arguments: &Value) -> Result<Reply, String> {
        let dir = project_dir(arguments)?;
        let asset = arguments
            .get("asset")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or("`asset` is required: the sequence's id")?;
        let change = SequenceChange {
            stills: stills(arguments)?,
            hold: match arguments.get("hold") {
                None => None,
                Some(hold) => Some(Frames(
                    hold.as_u64()
                        .ok_or("`hold` is a whole number of frames, at least 1")?,
                )),
            },
            looping: match arguments.get("loop") {
                None => None,
                Some(looping) => Some(looping.as_bool().ok_or("`loop` is true or false")?),
            },
        };
        let mut project = load(&dir)?;
        let id = AssetId::new(asset);
        let changed =
            change_sequence(&mut project, &id, change).map_err(|error| error.to_string())?;
        project
            .save(&dir)
            .map_err(|error| format!("saving the project: {error}"))?;
        Ok(match changed.before {
            None => format!("{id} — made: {}", changed.after),
            Some(before) => format!("{id} — was {before}; now {}", changed.after),
        }
        .into())
    }
}

/// The still ids, if a list was given.
fn stills(arguments: &Value) -> Result<Option<Vec<AssetId>>, String> {
    let Some(value) = arguments.get("stills") else {
        return Ok(None);
    };
    let refused = || "`stills` is a list of image asset ids".to_owned();
    value
        .as_array()
        .ok_or_else(refused)?
        .iter()
        .map(|id| id.as_str().map(AssetId::new).ok_or_else(refused))
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}
