//! Adding an asset whose content is the document.

use super::AuthorError;
use crate::asset::{Asset, AssetId};
use crate::color::Rgba;
use crate::icon::Icon;
use crate::pool::asset_id_for;
use crate::project::Project;
use crate::shape::{Geometry, Shape};
use crate::text::TextStyle;

/// The whole content of an asset with no file behind it, one variant per kind.
///
/// A closed set rather than a bag of optional fields, because which fields a
/// kind requires is not a runtime question: a colour asset is a colour, and a
/// caption with a `fill` on it is not a caption with something extra — it is
/// a mistake the type refuses to hold.
#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    /// A string, and the look it is set in.
    Text {
        /// What it says.
        text: String,
        /// How it is drawn, or `None` to leave the style out of the document
        /// entirely — which is every default, and the honest way to write a
        /// caption nobody has styled yet.
        style: Option<TextStyle>,
    },
    /// A solid colour filling whatever raster the render is.
    Color(
        /// The colour it is.
        Rgba,
    ),
    /// An outline the render draws.
    Shape(
        /// The outline, and how it is coloured.
        Shape,
    ),
    /// A symbol from the set this build ships.
    Icon(
        /// Which symbol, how big, in what colour.
        Icon,
    ),
}

impl Inline {
    /// The id this content asks to be called when the caller names none.
    ///
    /// Legible on purpose, because an id is what a clip is written against and
    /// what a person reads in the document afterwards: a caption is named for
    /// its own opening words, a shape for its outline, an icon for its symbol.
    /// Sanitising and suffixing happen after this, in [`asset_id_for`].
    fn suggests(&self) -> String {
        match self {
            Self::Text { text, .. } => opening_words(text),
            Self::Color(_) => "color".to_owned(),
            Self::Shape(shape) => match shape.geometry {
                Geometry::Rectangle { .. } => "rectangle",
                Geometry::Ellipse { .. } => "ellipse",
                Geometry::Arrow { .. } => "arrow",
            }
            .to_owned(),
            Self::Icon(icon) => icon.name.clone(),
        }
    }

    /// The asset this content makes, under the id decided for it.
    fn asset(self, id: AssetId) -> Asset {
        match self {
            Self::Text { text, style } => Asset {
                style,
                ..Asset::text(id, text)
            },
            Self::Color(color) => Asset::color(id, color),
            Self::Shape(shape) => Asset::shape(id, shape),
            Self::Icon(icon) => Asset::icon(id, icon),
        }
    }
}

/// How much of a caption becomes its id. Long enough to tell two lines of
/// narration apart at a glance, short enough that a clip written against it
/// still fits on a line.
const ID_WORDS: usize = 4;

/// The first few words of a string, hyphenated — `"The vessel arrives at
/// dawn"` becomes `the-vessel-arrives-at`.
fn opening_words(text: &str) -> String {
    let words: Vec<&str> = text.split_whitespace().take(ID_WORDS).collect();
    if words.is_empty() {
        return "text".to_owned();
    }
    words.join("-")
}

/// Writes one inline asset into the table, and hands back the id it got.
///
/// `id` is taken as written and a repeat is refused; left out, one is derived
/// from the content and suffixed until it is free. The distinction is the
/// point: a caller that named an id is about to write it onto a clip, so
/// handing back a different one silently would be worse than refusing.
pub fn add_asset(
    project: &mut Project,
    id: Option<&str>,
    content: Inline,
) -> Result<AssetId, AuthorError> {
    let id = match id {
        Some(asked) => {
            let id = AssetId::new(asked.trim());
            if project.assets.iter().any(|asset| asset.id == id) {
                return Err(AuthorError::TakenAssetId { id });
            }
            id
        }
        None => asset_id_for(project, &content.suggests()),
    };

    let mut proposed = project.clone();
    proposed.assets.push(content.asset(id.clone()));
    proposed.validate()?;
    *project = proposed;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::Fps;

    fn project() -> Project {
        Project::new("T", Fps::new(30, 1).expect("30fps"))
    }

    fn caption(text: &str) -> Inline {
        Inline::Text {
            text: text.to_owned(),
            style: None,
        }
    }

    #[test]
    fn an_unnamed_caption_is_named_for_its_opening_words() {
        let mut project = project();
        let id = add_asset(
            &mut project,
            None,
            caption("The vessel arrives at dawn, alone"),
        )
        .expect("a caption is a valid asset");
        assert_eq!(id.as_str(), "the-vessel-arrives-at");
        assert_eq!(project.assets.len(), 1);
    }

    /// Two identical captions is an ordinary thing to write — the same word
    /// twice in a cut — so the second is suffixed rather than refused.
    #[test]
    fn a_derived_id_that_is_taken_is_suffixed() {
        let mut project = project();
        add_asset(&mut project, None, caption("DAWN")).expect("first");
        let second = add_asset(&mut project, None, caption("DAWN")).expect("second");
        assert_eq!(second.as_str(), "dawn-2");
    }

    /// The other half of that rule: an id somebody chose is never quietly
    /// changed, because they are about to write it onto a clip.
    #[test]
    fn a_named_id_that_is_taken_is_refused_and_writes_nothing() {
        let mut project = project();
        add_asset(&mut project, Some("card"), Inline::Color(Rgba::BLACK)).expect("first");
        let again = add_asset(&mut project, Some("card"), Inline::Color(Rgba::WHITE));
        assert!(matches!(again, Err(AuthorError::TakenAssetId { .. })));
        assert_eq!(project.assets.len(), 1);
    }

    /// The all-or-nothing rule, with the cheapest document validation refuses:
    /// a shape with neither a fill nor a border draws nothing at all.
    #[test]
    fn a_shape_that_would_draw_nothing_leaves_the_table_empty() {
        let mut project = project();
        let blank = Shape {
            geometry: Geometry::Ellipse {
                width: 0.2,
                height: 0.2,
            },
            fill: None,
            stroke: None,
            stroke_width: 0.004,
        };
        let refused = add_asset(&mut project, None, Inline::Shape(blank));
        assert!(matches!(refused, Err(AuthorError::Refused(_))));
        assert!(project.assets.is_empty());
    }

    /// A caption authored with a look keeps it. Dropping the style on the way
    /// into the table would render a title in the wrong face and size, and
    /// nothing about the document would say so.
    #[test]
    fn a_style_asked_for_is_the_style_that_lands() {
        let mut project = project();
        let style = TextStyle {
            size: 0.06,
            ..TextStyle::default()
        };
        let id = add_asset(
            &mut project,
            None,
            Inline::Text {
                text: "DAWN".to_owned(),
                style: Some(style),
            },
        )
        .expect("a styled caption");
        let asset = project.asset(&id).expect("it was written");
        assert_eq!(asset.style.as_ref().map(|style| style.size), Some(0.06));
    }

    #[test]
    fn an_icon_is_named_for_its_symbol() {
        let mut project = project();
        let id = add_asset(
            &mut project,
            None,
            Inline::Icon(Icon::new("clapperboard", 0.2, Rgba::WHITE)),
        )
        .expect("an icon is a valid asset");
        assert_eq!(id.as_str(), "clapperboard");
    }
}
