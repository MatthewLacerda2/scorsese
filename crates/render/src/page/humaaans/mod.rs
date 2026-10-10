//! The people a page is given: Pablo Stanley's **Humaaans** (CC0), as parts.
//!
//! Served at `https://lib.scorsese/humaaans/<kind>/<name>.svg` (#1004), beside
//! the icons and the fonts, and listed at `humaaans/index.json`. A person is
//! three parts stacked in one frame — a `head`, a lower half (`standing` or
//! `sitting`) and a `body` over both — so a page builds one from parts rather
//! than drawing a stick figure, and two videos need not show the same person.
//! The `person` kind is the pack's own 32 assembled people, the `seat` and
//! `scene` kinds its props, each in its own frame.
//!
//! The official files are compiled in **as downloaded** (their folder's
//! `README.md` has where from), and rewritten on request ([`svg`]): Sketch's
//! comments and stroke settings dropped, the parts placed in the shared frame,
//! and every colour that means skin, hair or clothing made a CSS variable with
//! the pack's own colour as its fallback. So an untouched part draws exactly as
//! the artist drew it, and a page recolours one person by setting variables on
//! its group.

mod svg;

/// The frame every `head`, `body`, `standing` and `sitting` part is served in:
/// stacked in it, in that order with the body last, they make one person.
pub(crate) const FRAME: (u32, u32) = (300, 426);

/// One file of the pack.
struct Part {
    /// What it is: `head`, `body`, `standing`, `sitting`, `seat`, `scene`
    /// or `person`.
    kind: &'static str,
    name: &'static str,
    /// The official file, unchanged.
    source: &'static str,
}

macro_rules! part {
    ($kind:literal, $name:literal, $file:literal) => {
        Part {
            kind: $kind,
            name: $name,
            source: include_str!(concat!("../shipped/humaaans/official/", $file)),
        }
    };
}

/// Every file served, in the order the index lists them.
const PARTS: &[Part] = &[
    part!("head", "afro", "Single Pieces/Head/Front/Afro.svg"),
    part!("head", "airy", "Single Pieces/Head/Front/Airy.svg"),
    part!("head", "caesar", "Single Pieces/Head/Front/Caesar.svg"),
    part!("head", "chongo", "Single Pieces/Head/Front/Chongo.svg"),
    part!("head", "curly", "Single Pieces/Head/Front/Curly.svg"),
    part!("head", "hijab-1", "Single Pieces/Head/Front/Hijab 1.svg"),
    part!("head", "hijab-2", "Single Pieces/Head/Front/Hijab2.svg"),
    part!("head", "long", "Single Pieces/Head/Front/Long.svg"),
    part!("head", "no-hair", "Single Pieces/Head/Front/No Hair.svg"),
    part!("head", "pony", "Single Pieces/Head/Front/Pony.svg"),
    part!("head", "rad", "Single Pieces/Head/Front/Rad.svg"),
    part!("head", "short-1", "Single Pieces/Head/Front/Short 1.svg"),
    part!("head", "short-2", "Single Pieces/Head/Front/Short 2.svg"),
    part!(
        "head",
        "short-beard",
        "Single Pieces/Head/Front/Short Beard.svg"
    ),
    part!("head", "top", "Single Pieces/Head/Front/Top.svg"),
    part!("head", "turban-1", "Single Pieces/Head/Front/Turban 1.svg"),
    part!("head", "turban-2", "Single Pieces/Head/Front/Turban2.svg"),
    part!("head", "wavy", "Single Pieces/Head/Front/Wavy.svg"),
    part!("body", "hoodie", "Single Pieces/Body/Hoodie.svg"),
    part!("body", "jacket-2", "Single Pieces/Body/Jacket 2.svg"),
    part!("body", "jacket", "Single Pieces/Body/Jacket.svg"),
    part!("body", "lab-coat", "Single Pieces/Body/Lab Coat.svg"),
    part!("body", "long-sleeve", "Single Pieces/Body/Long Sleeve.svg"),
    part!(
        "body",
        "pointing-forward",
        "Single Pieces/Body/Pointing Forward.svg"
    ),
    part!("body", "pointing-up", "Single Pieces/Body/Pointing Up.svg"),
    part!("body", "pregnant", "Single Pieces/Body/Pregnant.svg"),
    part!("body", "trench-coat", "Single Pieces/Body/Trench Coat.svg"),
    part!("body", "turtle-neck", "Single Pieces/Body/Turtle Neck.svg"),
    part!(
        "standing",
        "baggy-pants",
        "Single Pieces/Bottom/Standing/Baggy Pants.svg"
    ),
    part!(
        "standing",
        "jogging",
        "Single Pieces/Bottom/Standing/Jogging.svg"
    ),
    part!(
        "standing",
        "shorts",
        "Single Pieces/Bottom/Standing/Shorts.svg"
    ),
    part!(
        "standing",
        "skinny-jeans-walk",
        "Single Pieces/Bottom/Standing/Skinny Jeans Walk.svg"
    ),
    part!(
        "standing",
        "skinny-jeans",
        "Single Pieces/Bottom/Standing/Skinny Jeans.svg"
    ),
    part!(
        "standing",
        "skirt",
        "Single Pieces/Bottom/Standing/Skirt.svg"
    ),
    part!(
        "standing",
        "sprint",
        "Single Pieces/Bottom/Standing/Sprint.svg"
    ),
    part!(
        "standing",
        "sweatpants",
        "Single Pieces/Bottom/Standing/Sweatpants.svg"
    ),
    part!(
        "sitting",
        "baggy-pants",
        "Single Pieces/Bottom/Sitting/Baggy Pants.svg"
    ),
    part!(
        "sitting",
        "skinny-jeans",
        "Single Pieces/Bottom/Sitting/Skinny Jeans 1.svg"
    ),
    part!(
        "sitting",
        "sweat-pants",
        "Single Pieces/Bottom/Sitting/Sweat Pants.svg"
    ),
    part!(
        "sitting",
        "wheelchair",
        "Single Pieces/Bottom/Sitting/Wheelchair.svg"
    ),
    part!("seat", "ball", "Single Pieces/Objects/Seat/Ball.svg"),
    part!("seat", "cube-2", "Single Pieces/Objects/Seat/Cube 2.svg"),
    part!("seat", "cube", "Single Pieces/Objects/Seat/Cube.svg"),
    part!("scene", "home", "Single Pieces/Scene/Home.svg"),
    part!("scene", "plants", "Single Pieces/Scene/Plants.svg"),
    part!("scene", "whiteboard", "Single Pieces/Scene/Whiteboard.svg"),
    part!("scene", "wireframe", "Single Pieces/Scene/Wireframe.svg"),
    part!("person", "sitting-1", "Humaaans/sitting-1.svg"),
    part!("person", "sitting-2", "Humaaans/sitting-2.svg"),
    part!("person", "sitting-3", "Humaaans/sitting-3.svg"),
    part!("person", "sitting-4", "Humaaans/sitting-4.svg"),
    part!("person", "sitting-5", "Humaaans/sitting-5.svg"),
    part!("person", "sitting-6", "Humaaans/sitting-6.svg"),
    part!("person", "sitting-7", "Humaaans/sitting-7.svg"),
    part!("person", "sitting-8", "Humaaans/sitting-8.svg"),
    part!("person", "standing-1", "Humaaans/standing-1.svg"),
    part!("person", "standing-2", "Humaaans/standing-2.svg"),
    part!("person", "standing-3", "Humaaans/standing-3.svg"),
    part!("person", "standing-4", "Humaaans/standing-4.svg"),
    part!("person", "standing-5", "Humaaans/standing-5.svg"),
    part!("person", "standing-6", "Humaaans/standing-6.svg"),
    part!("person", "standing-7", "Humaaans/standing-7.svg"),
    part!("person", "standing-8", "Humaaans/standing-8.svg"),
    part!("person", "standing-9", "Humaaans/standing-9.svg"),
    part!("person", "standing-10", "Humaaans/standing-10.svg"),
    part!("person", "standing-11", "Humaaans/standing-11.svg"),
    part!("person", "standing-12", "Humaaans/standing-12.svg"),
    part!("person", "standing-13", "Humaaans/standing-13.svg"),
    part!("person", "standing-14", "Humaaans/standing-14.svg"),
    part!("person", "standing-15", "Humaaans/standing-15.svg"),
    part!("person", "standing-16", "Humaaans/standing-16.svg"),
    part!("person", "standing-17", "Humaaans/standing-17.svg"),
    part!("person", "standing-18", "Humaaans/standing-18.svg"),
    part!("person", "standing-19", "Humaaans/standing-19.svg"),
    part!("person", "standing-20", "Humaaans/standing-20.svg"),
    part!("person", "standing-21", "Humaaans/standing-21.svg"),
    part!("person", "standing-22", "Humaaans/standing-22.svg"),
    part!("person", "standing-23", "Humaaans/standing-23.svg"),
    part!("person", "standing-24", "Humaaans/standing-24.svg"),
];

/// Where a kind's parts sit in [`FRAME`], or `None` for a kind served in its
/// own frame. The pack's own offsets, as every assembled person places them.
fn offset(kind: &str) -> Option<(u32, u32)> {
    match kind {
        "head" => Some((82, 0)),
        "body" => Some((22, 82)),
        "standing" | "sitting" => Some((0, 187)),
        _ => None,
    }
}

/// What the `humaaans/` folder answers for `file`, the path under it: the
/// part's document, or the page note for a part it does not ship.
pub(crate) fn serve(file: &str) -> Result<Vec<u8>, String> {
    if file == "index.json" {
        return Ok(index());
    }
    let wanted = file.strip_suffix(".svg").unwrap_or(file);
    let found = wanted.split_once('/').and_then(|(kind, name)| {
        PARTS
            .iter()
            .find(|part| part.kind == kind && part.name == name)
    });
    match found {
        Some(part) => {
            let prefix = format!("humaaans-{}-{}", part.kind, part.name);
            Ok(svg::rewrite(part.source, offset(part.kind), &prefix).into_bytes())
        }
        None => Err(unknown(wanted)),
    }
}

/// The page note for a part the pack does not have: the names of its kind,
/// or the kinds when the kind itself is wrong.
fn unknown(wanted: &str) -> String {
    let kind = wanted.split_once('/').map_or("", |(kind, _)| kind);
    let names: Vec<_> = PARTS
        .iter()
        .filter(|part| part.kind == kind)
        .map(|part| part.name)
        .collect();
    let choices = if names.is_empty() {
        let mut kinds: Vec<_> = PARTS.iter().map(|part| part.kind).collect();
        kinds.dedup();
        format!("the kinds are {}", kinds.join(", "))
    } else {
        format!("the {kind} parts are {}", names.join(", "))
    };
    format!(
        "the page asked for the Humaaans part `{wanted}`, which scorsese does not ship ({choices}; \
         https://lib.scorsese/humaaans/index.json lists them). It rendered without it"
    )
}

/// The colour regions a page can set, as `--person-<region>`. Each `-shade`
/// is the darker side of the one before it: the back arm, the far leg.
pub(crate) const REGIONS: &[&str] = &[
    "skin",
    "skin-shade",
    "hair",
    "headwear",
    "headwear-shade",
    "top",
    "top-shade",
    "shirt",
    "bottom",
    "bottom-shade",
    "shoes",
    "seat",
];

/// `humaaans/index.json`: the frame, each kind's names, the regions, and the
/// licence, so a page or an assistant knows what exists without guessing.
fn index() -> Vec<u8> {
    let mut kinds = serde_json::Map::new();
    for part in PARTS {
        let names = kinds
            .entry(part.kind)
            .or_insert_with(|| serde_json::Value::Array(Vec::new()));
        if let serde_json::Value::Array(names) = names {
            names.push(part.name.into());
        }
    }
    let index = serde_json::json!({
        "frame": { "width": FRAME.0, "height": FRAME.1 },
        "stack": ["head", "standing or sitting", "body"],
        "parts": kinds,
        "regions": REGIONS,
        "licence": "CC0 1.0 (public domain), Humaaans by Pablo Stanley, humaaans.com",
    });
    serde_json::to_vec(&index).expect("a tree of plain values serialises")
}

#[cfg(test)]
mod tests;
