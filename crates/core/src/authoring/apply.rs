//! Writing an edit onto the asset it names, field by field.
//!
//! Split from its neighbour because the two are different jobs: [`super::edit`]
//! decides whether a change is one this asset can take, and this decides what
//! the document then says. Every function here has already been told the field
//! belongs to the kind.

use std::fmt;

use super::AuthorError;
use super::edit::Edit;
use crate::asset::{Asset, AssetKind};
use crate::shape::Geometry;
use crate::text::{FontChoice, TextAlign};

/// Writes the edit onto the asset, collecting a line per field changed.
pub(super) fn apply(asset: &mut Asset, edit: &Edit) -> Result<Vec<String>, AuthorError> {
    let mut said = Vec::new();
    match asset.kind {
        AssetKind::Text => text(asset, edit, &mut said),
        AssetKind::Color => {
            if let Some(color) = edit.color {
                said.push(became("color", show(asset.color), color));
                asset.color = Some(color);
            }
        }
        AssetKind::Shape => shape(asset, edit, &mut said)?,
        AssetKind::Icon => icon(asset, edit, &mut said)?,
        kind => {
            return Err(AuthorError::NotInline {
                asset: asset.id.clone(),
                kind,
            });
        }
    }
    Ok(said)
}

/// The string, and the style it is set in.
fn text(asset: &mut Asset, edit: &Edit, said: &mut Vec<String>) {
    if let Some(text) = &edit.text {
        said.push(became(
            "text",
            format!("{:?}", asset.text.as_deref().unwrap_or_default()),
            format!("{text:?}"),
        ));
        asset.text = Some(text.clone());
    }
    // Read back with its defaults filled in, so an asset that carried no style
    // at all is edited from what it actually looks like rather than from
    // nothing — which is what makes one named field change one thing.
    let mut style = asset.text_style();
    let mut styled = false;
    if let Some(font) = &edit.font {
        said.push(became("font", show_font(&style.font), font.clone()));
        style.font = FontChoice::from(font.clone());
        styled = true;
    }
    if let Some(weight) = edit.weight {
        said.push(became("weight", show(style.weight), weight));
        style.weight = Some(weight);
        styled = true;
    }
    if let Some(italic) = edit.italic {
        said.push(became("italic", style.italic, italic));
        style.italic = italic;
        styled = true;
    }
    if let Some(size) = edit.size {
        said.push(became("size", style.size, size));
        style.size = size;
        styled = true;
    }
    if let Some(color) = edit.color {
        said.push(became("color", style.color, color));
        style.color = color;
        styled = true;
    }
    if let Some(align) = edit.align {
        said.push(became("align", show_align(style.align), show_align(align)));
        style.align = align;
        styled = true;
    }
    if let Some(line_height) = edit.line_height {
        said.push(became("line_height", style.line_height, line_height));
        style.line_height = line_height;
        styled = true;
    }
    if let Some(max_width) = edit.max_width {
        said.push(became("max_width", style.max_width, max_width));
        style.max_width = max_width;
        styled = true;
    }
    if let Some(stroke) = edit.stroke {
        said.push(became("stroke", show(style.stroke), stroke));
        style.stroke = Some(stroke);
        styled = true;
    }
    if let Some(width) = edit.stroke_width {
        said.push(became("stroke_width", style.stroke_width, width));
        style.stroke_width = width;
        styled = true;
    }
    if styled {
        asset.style = Some(style);
    }
}

/// The colours a shape is drawn in, and the numbers of its outline.
fn shape(asset: &mut Asset, edit: &Edit, said: &mut Vec<String>) -> Result<(), AuthorError> {
    let id = asset.id.clone();
    let shape = asset.shape.as_mut().ok_or(AuthorError::Incoherent {
        asset: id,
        kind: AssetKind::Shape,
    })?;
    if let Some(fill) = edit.fill {
        said.push(became("fill", show(shape.fill), fill));
        shape.fill = Some(fill);
    }
    if let Some(stroke) = edit.stroke {
        said.push(became("stroke", show(shape.stroke), stroke));
        shape.stroke = Some(stroke);
    }
    if let Some(width) = edit.stroke_width {
        said.push(became("stroke_width", shape.stroke_width, width));
        shape.stroke_width = width;
    }
    geometry(&mut shape.geometry, edit, said)
}

/// A closed shape's own measurements. An arrow has none of them, and says so
/// rather than taking a number nothing would read back.
fn geometry(
    outline: &mut Geometry,
    edit: &Edit,
    said: &mut Vec<String>,
) -> Result<(), AuthorError> {
    let asked = edit.width.or(edit.height).or(edit.radius);
    if asked.is_none() {
        return Ok(());
    }
    let (width, height, radius) = match outline {
        Geometry::Rectangle {
            width,
            height,
            radius,
        } => (width, height, Some(radius)),
        Geometry::Ellipse { width, height } => (width, height, None),
        Geometry::Arrow { .. } => {
            return Err(AuthorError::NotOnGeometry {
                field: measurement(edit),
                geometry: "an arrow",
                because: "an arrow has two endpoints rather than a size, and moving one \
                          is a project_write",
            });
        }
    };
    if let Some(new) = edit.width {
        said.push(became("width", *width, new));
        *width = new;
    }
    if let Some(new) = edit.height {
        said.push(became("height", *height, new));
        *height = new;
    }
    if let Some(new) = edit.radius {
        let corner = radius.ok_or(AuthorError::NotOnGeometry {
            field: "radius",
            geometry: "an ellipse",
            because: "an ellipse is all corner already",
        })?;
        said.push(became("radius", *corner, new));
        *corner = new;
    }
    Ok(())
}

/// Which measurement a caller named, for a refusal that has to name one.
fn measurement(edit: &Edit) -> &'static str {
    if edit.width.is_some() {
        "width"
    } else if edit.height.is_some() {
        "height"
    } else {
        "radius"
    }
}

/// Which symbol, how big, in what colour, how thick.
fn icon(asset: &mut Asset, edit: &Edit, said: &mut Vec<String>) -> Result<(), AuthorError> {
    let id = asset.id.clone();
    let icon = asset.icon.as_mut().ok_or(AuthorError::Incoherent {
        asset: id,
        kind: AssetKind::Icon,
    })?;
    if let Some(name) = &edit.icon {
        said.push(became("icon", icon.name.clone(), name.clone()));
        icon.name.clone_from(name);
    }
    if let Some(size) = edit.size {
        said.push(became("size", icon.size, size));
        icon.size = size;
    }
    if let Some(color) = edit.color {
        said.push(became("color", icon.color, color));
        icon.color = color;
    }
    if let Some(width) = edit.stroke_width {
        said.push(became("stroke_width", icon.stroke_width, width));
        icon.stroke_width = width;
    }
    Ok(())
}

/// One line of the answer: which field, what it was, what it is now.
///
/// Both halves, because the caller cannot see the document — and *was* is what
/// says a change landed on the value it was aimed at rather than on one
/// something else had already moved.
fn became(field: &str, was: impl fmt::Display, now: impl fmt::Display) -> String {
    format!("{field}: {was} → {now}")
}

/// An optional value as it reads in that line — an absent one is a real answer
/// and has to say so.
fn show(value: Option<impl fmt::Display>) -> String {
    value.map_or_else(|| "none".to_owned(), |value| value.to_string())
}

fn show_font(font: &FontChoice) -> String {
    String::from(font.clone())
}

fn show_align(align: TextAlign) -> &'static str {
    match align {
        TextAlign::Left => "left",
        TextAlign::Center => "center",
        TextAlign::Right => "right",
    }
}

#[cfg(test)]
mod tests {
    use super::super::fixture::project;
    use super::super::{Edit, set_asset};
    use crate::asset::AssetId;
    use crate::color::Rgba;
    use crate::shape::{Endpoint, Geometry, Point, Shape};

    #[test]
    fn a_colour_card_takes_its_one_field() {
        let mut project = project();
        let said = set_asset(
            &mut project,
            &AssetId::new("card"),
            &Edit {
                color: Some(Rgba::opaque(0xff, 0xcc, 0)),
                ..Edit::default()
            },
        )
        .expect("a repainted card");
        assert_eq!(said, vec!["color: #101820 → #ffcc00"]);
    }

    #[test]
    fn a_box_takes_a_border_and_a_new_width_in_one_call() {
        let mut project = project();
        let said = set_asset(
            &mut project,
            &AssetId::new("box"),
            &Edit {
                stroke: Some(Rgba::BLACK),
                width: Some(0.5),
                ..Edit::default()
            },
        )
        .expect("a bordered box");
        assert_eq!(said, vec!["stroke: none → #000000", "width: 0.4 → 0.5"]);
    }

    /// An arrow is two endpoints and has no size at all, so a width on one is
    /// refused by name rather than written where nothing reads it back.
    #[test]
    fn an_arrow_has_no_width() {
        let mut project = project();
        let arrow = Shape::outlined(
            Geometry::Arrow {
                from: Endpoint::from(Point::new(0.1, 0.1)),
                to: Endpoint::from(Point::new(0.9, 0.9)),
                curve: crate::shape::Curve::Straight,
                heads: crate::shape::Heads::End,
            },
            Rgba::WHITE,
        );
        super::super::add_asset(
            &mut project,
            Some("pointer"),
            super::super::Inline::Shape(arrow),
        )
        .expect("an arrow is a valid asset");
        let refused = set_asset(
            &mut project,
            &AssetId::new("pointer"),
            &Edit {
                width: Some(0.3),
                ..Edit::default()
            },
        );
        let problem = refused.expect_err("an arrow has no width");
        let said = problem.to_string();
        assert!(said.contains("`width` is not a field"), "{said}");
        assert!(said.contains("an arrow"), "{said}");
    }

    #[test]
    fn an_icon_changes_symbol_and_colour_and_says_both() {
        let mut project = project();
        let said = set_asset(
            &mut project,
            &AssetId::new("mark"),
            &Edit {
                icon: Some("circle-play".to_owned()),
                color: Some(Rgba::BLACK),
                ..Edit::default()
            },
        )
        .expect("another symbol");
        assert_eq!(
            said,
            vec![
                "icon: clapperboard → circle-play",
                "color: #ffffff → #000000"
            ]
        );
    }
}
