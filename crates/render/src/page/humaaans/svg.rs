//! One official Humaaans file, rewritten for a page: what Sketch left in
//! dropped, the part placed in its frame, and its colours made settable.
//!
//! Sketch writes every file the same plain way — double-quoted attributes, no
//! entities, no text but a title — so a tag reader is all it takes, and the
//! tests run every file of the pack through it.
//!
//! **Which colour is which region** is read off the ids Sketch kept: the group
//! a fill sits in (`Head/…`, `Body/…`, `Bottom/…`, a shoe, a seat) and the
//! fill's own (`Coat-Back`, `Leg-Front`, `Shirt`). Skin is its two colours
//! everywhere; black and white are the pack's shading and highlights, laid
//! over whatever colour is under them, and are left alone, as is a scene.

/// An element, with what is inside it. Text, comments, `<title>` and `<desc>`
/// are not kept: nothing a page draws is in them.
struct Element {
    tag: String,
    attributes: Vec<(String, String)>,
    children: Vec<Element>,
}

impl Element {
    fn get(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    fn id(&self) -> &str {
        self.get("id").unwrap_or_default()
    }

    /// Itself and everything inside it, outermost first.
    fn all(&self) -> Vec<&Element> {
        let mut all = vec![self];
        all.extend(self.children.iter().flat_map(Element::all));
        all
    }
}

/// Reads the document's root element. Panics on a file that is not plain
/// Sketch output, which the tests would catch for every shipped one.
fn parse(source: &str) -> Element {
    let mut stack = vec![Element {
        tag: String::new(),
        attributes: Vec::new(),
        children: Vec::new(),
    }];
    let mut rest = source;
    while let Some(open) = rest.find('<') {
        rest = &rest[open..];
        if rest.starts_with("<!--") {
            rest = &rest[rest.find("-->").expect("a comment ends") + 3..];
            continue;
        }
        let close = rest.find('>').expect("a tag ends");
        let tag = &rest[1..close];
        rest = &rest[close + 1..];
        if tag.starts_with('?') {
            continue;
        }
        if let Some(name) = tag.strip_prefix('/') {
            let done = stack.pop().expect("an end tag has a start");
            assert_eq!(done.tag, name.trim(), "tags nest");
            stack
                .last_mut()
                .expect("the root is kept")
                .children
                .push(done);
            continue;
        }
        let empty = tag.ends_with('/');
        let tag = tag.trim_end_matches('/');
        let (name, mut attributes) = tag.split_once(char::is_whitespace).unwrap_or((tag, ""));
        let mut element = Element {
            tag: name.to_owned(),
            attributes: Vec::new(),
            children: Vec::new(),
        };
        while let Some((key, after)) = attributes.split_once("=\"") {
            let (value, after) = after.split_once('"').expect("a value is quoted");
            element
                .attributes
                .push((key.trim().to_owned(), value.to_owned()));
            attributes = after;
        }
        if empty {
            stack
                .last_mut()
                .expect("the root is kept")
                .children
                .push(element);
        } else {
            stack.push(element);
        }
    }
    let mut document = stack.pop().expect("the document is kept");
    document.children.pop().expect("a file has an <svg>")
}

/// What a fill sits inside, from the ids of the groups around it.
#[derive(Clone, Copy)]
enum Within {
    Nothing,
    Head,
    /// A top, and whether it has a coat over its shirt.
    Body {
        coat: bool,
    },
    /// A lower half, and which of its two clothing colours is the darker.
    Bottom {
        shade: Option<u32>,
    },
    Shoe,
    Seat,
}

impl Within {
    /// Where `element` puts what is inside it, from where it sits.
    fn enter(self, element: &Element) -> Self {
        let id = element.id();
        if id.starts_with("Head/") {
            Within::Head
        } else if id.starts_with("Body/") {
            let coat = element.all().iter().any(|inner| inner.id() == "Coat-Front");
            Within::Body { coat }
        } else if id.starts_with("Bottom/") {
            Within::Bottom {
                shade: darker_clothing(element),
            }
        } else if id.starts_with("Accessories/Shoe") {
            Within::Shoe
        } else if id.starts_with("Objects/") || id == "Seat-Stuff" {
            Within::Seat
        } else {
            self
        }
    }
}

/// A `#RRGGBB` as a number.
fn colour(fill: &str) -> Option<u32> {
    let hex = fill.strip_prefix('#').filter(|hex| hex.len() == 6)?;
    u32::from_str_radix(hex, 16).ok()
}

/// How light a colour looks, near enough to tell a shade from its colour.
fn lightness(rgb: u32) -> u32 {
    let [_, r, g, b] = rgb.to_be_bytes();
    299 * u32::from(r) + 587 * u32::from(g) + 114 * u32::from(b)
}

const SKIN: u32 = 0xB2_8B67;
const SKIN_SHADE: u32 = 0x99_7659;
const HAIR: u32 = 0x19_1847;

/// The darker of a lower half's two clothing colours, when its legs are two
/// fills under one name (`Pant`, `Leg`, `Bottom`) and only the colour says
/// which is the far one.
fn darker_clothing(bottom: &Element) -> Option<u32> {
    let mut colours: Vec<u32> = bottom
        .all()
        .iter()
        .filter(|inner| matches!(inner.id(), "Pant" | "Leg" | "Bottom"))
        .filter_map(|inner| colour(inner.get("fill")?))
        .filter(|rgb| ![SKIN, SKIN_SHADE].contains(rgb))
        .collect();
    colours.sort_unstable();
    colours.dedup();
    (colours.len() == 2).then(|| {
        *colours
            .iter()
            .min_by_key(|rgb| lightness(**rgb))
            .expect("two")
    })
}

/// The region a fill of `rgb` on an element named `id` paints, or `None` for
/// one a page does not recolour.
fn region(within: Within, id: &str, rgb: u32) -> Option<&'static str> {
    match (rgb, within) {
        (SKIN, _) => Some("skin"),
        (SKIN_SHADE, _) => Some("skin-shade"),
        (0x00_0000 | 0xFF_FFFF, _) | (_, Within::Nothing) => None,
        (_, Within::Shoe) => Some("shoes"),
        (_, Within::Seat) => Some("seat"),
        (HAIR, Within::Head) => Some("hair"),
        (0x2C_2C2C, Within::Head) => Some("headwear-shade"),
        (_, Within::Head) => Some("headwear"),
        (_, Within::Body { coat }) => Some(match id {
            "Shirt" if coat => "shirt",
            "Sleeve" => "top-shade",
            _ if id.contains("Back") => "top-shade",
            _ => "top",
        }),
        (_, Within::Bottom { shade }) => match id {
            // The wheelchair is a chair, not clothing.
            "Base" | "Seat" | "Wheel-Stuff" | "Front-Wheel" | "Wheel" => None,
            "Leg-Back" | "LegLower" | "Skirt-Shadow" => Some("bottom-shade"),
            _ if shade == Some(rgb) => Some("bottom-shade"),
            _ => Some("bottom"),
        },
    }
}

/// Writes `element` out as a page gets it, its referenced ids begun with
/// `prefix`: every turban's mask is `mask-2`, and two parts on one page must
/// not point at each other's.
fn write(element: &Element, within: Within, prefix: &str, out: &mut String) {
    if matches!(element.tag.as_str(), "title" | "desc") {
        return;
    }
    let within = within.enter(element);
    out.push('<');
    out.push_str(&element.tag);
    for (key, value) in &element.attributes {
        // Sketch's ids are names in its layer list, repeated in every part; a
        // page would find the wrong one. Only those something points at stay.
        let kept = match key.as_str() {
            "id" => is_referenced(value),
            "stroke" | "stroke-width" => value != "none" && value != "1",
            _ => true,
        };
        if !kept {
            continue;
        }
        let key = if key == "xlink:href" {
            "href"
        } else {
            key.as_str()
        };
        let value = match key {
            "id" => format!("{prefix}-{value}"),
            "href" => value.replacen('#', &format!("#{prefix}-"), 1),
            _ => value.replace("url(#", &format!("url(#{prefix}-")),
        };
        out.push_str(&format!(" {key}=\"{value}\""));
    }
    if let Some((fill, rgb)) = element
        .get("fill")
        .and_then(|fill| Some((fill, colour(fill)?)))
        && let Some(region) = region(within, element.id(), rgb)
    {
        out.push_str(&format!(" style=\"fill:var(--person-{region},{fill})\""));
    }
    out.push('>');
    for child in &element.children {
        write(child, within, prefix, out);
    }
    out.push_str(&format!("</{}>", element.tag));
}

/// Whether an id is one a mask or a `<use>` points at (`path-1`, `mask-2`).
fn is_referenced(id: &str) -> bool {
    id.starts_with("path-") || id.starts_with("mask-")
}

/// The official file `source` as a page gets it: in its own frame, or moved
/// by `offset` into the person frame ([`super::FRAME`]), with `prefix` naming
/// it in the ids it keeps.
pub(super) fn rewrite(source: &str, offset: Option<(u32, u32)>, prefix: &str) -> String {
    let svg = parse(source);
    let (width, height) = match offset {
        Some(_) => super::FRAME,
        None => {
            let size = |name| {
                let value = svg.get(name).expect("Sketch sizes its files");
                value
                    .trim_end_matches("px")
                    .parse()
                    .expect("a whole number")
            };
            (size("width"), size("height"))
        }
    };
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" \
         viewBox=\"0 0 {width} {height}\">"
    );
    if let Some((x, y)) = offset {
        out.push_str(&format!("<g transform=\"translate({x} {y})\">"));
    }
    for child in &svg.children {
        write(child, Within::Nothing, prefix, &mut out);
    }
    if offset.is_some() {
        out.push_str("</g>");
    }
    out.push_str("</svg>");
    out
}
