//! Changing one field of an inline asset, and leaving the rest alone.
//!
//! **Merged, never replaced, and that is the whole design.** The loop this
//! exists for is *reword the caption* and *drop its size by a hundredth* — one
//! field at a time, several times over, while looking at stills. A verb that
//! took a whole `style` block would silently lose every field the caller did
//! not restate, so the second call to shrink a title would also reset the font
//! somebody chose two turns earlier, and nothing would say so. An absent
//! argument here means *unchanged*, and it is the only thing it could safely
//! mean.

use super::AuthorError;
use super::apply::apply;
use crate::asset::{AssetId, AssetKind};
use crate::color::Rgba;
use crate::project::Project;
use crate::text::TextAlign;

/// One change to an inline asset. Every field is optional and an absent one is
/// left exactly as it is.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Edit {
    /// What a `text` asset says.
    pub text: Option<String>,
    /// The face to set it in: a shipped name, or a font file in the project.
    pub font: Option<String>,
    /// How heavy the glyphs are, on OpenType's `wght` scale.
    pub weight: Option<u16>,
    /// Whether to set it in the family's italic.
    pub italic: Option<bool>,
    /// How big, as a fraction of the frame's height — a `text` or an `icon`.
    pub size: Option<f64>,
    /// The one colour a `text`, `color` or `icon` asset is drawn in.
    pub color: Option<Rgba>,
    /// Which edge the lines of a `text` asset line up against.
    pub align: Option<TextAlign>,
    /// Baseline to baseline, as a multiple of the size.
    pub line_height: Option<f64>,
    /// How wide the text runs before it wraps, as a fraction of the frame.
    pub max_width: Option<f64>,
    /// Which symbol an `icon` asset draws — its `name` field.
    pub icon: Option<String>,
    /// What a `shape`'s interior is painted.
    pub fill: Option<Rgba>,
    /// The rim: a `shape`'s border, or the outline a `text` asset carries
    /// outside its letterforms.
    pub stroke: Option<Rgba>,
    /// How thick that rim is, or an `icon`'s own line.
    pub stroke_width: Option<f64>,
    /// A closed shape's width, as a fraction of the raster's width.
    pub width: Option<f64>,
    /// A closed shape's height, as a fraction of the raster's height.
    pub height: Option<f64>,
    /// How rounded a rectangle's corners are.
    pub radius: Option<f64>,
}

impl Edit {
    /// The fields this edit names, in the order they are declared.
    fn named(&self) -> Vec<&'static str> {
        let asked: [(&'static str, bool); 16] = [
            ("text", self.text.is_some()),
            ("font", self.font.is_some()),
            ("weight", self.weight.is_some()),
            ("italic", self.italic.is_some()),
            ("size", self.size.is_some()),
            ("color", self.color.is_some()),
            ("align", self.align.is_some()),
            ("line_height", self.line_height.is_some()),
            ("max_width", self.max_width.is_some()),
            ("icon", self.icon.is_some()),
            ("fill", self.fill.is_some()),
            ("stroke", self.stroke.is_some()),
            ("stroke_width", self.stroke_width.is_some()),
            ("width", self.width.is_some()),
            ("height", self.height.is_some()),
            ("radius", self.radius.is_some()),
        ];
        asked
            .into_iter()
            .filter_map(|(field, asked)| asked.then_some(field))
            .collect()
    }
}

/// What a `text` asset takes. `stroke` and `stroke_width` are here as well as
/// on a shape: they are the same two words in the same unit, and the rim they
/// put on a caption is what makes it survive whatever is behind it.
const TEXT: &[&str] = &[
    "text",
    "font",
    "weight",
    "italic",
    "size",
    "color",
    "align",
    "line_height",
    "max_width",
    "stroke",
    "stroke_width",
];

/// What a `color` asset takes, which is the whole of what it is.
const COLOR: &[&str] = &["color"];

/// What a `shape` asset takes. The geometry's own numbers are here too — a
/// box's width is as ordinary a thing to nudge as its fill.
const SHAPE: &[&str] = &[
    "fill",
    "stroke",
    "stroke_width",
    "width",
    "height",
    "radius",
];

/// What an `icon` asset takes.
const ICON: &[&str] = &["icon", "size", "color", "stroke_width"];

/// The fields a kind takes, or `None` for a kind whose content is not in the
/// document at all.
fn fields_of(kind: AssetKind) -> Option<&'static [&'static str]> {
    match kind {
        AssetKind::Text => Some(TEXT),
        AssetKind::Color => Some(COLOR),
        AssetKind::Shape => Some(SHAPE),
        AssetKind::Icon => Some(ICON),
        _ => None,
    }
}

/// Changes the fields named on one inline asset, and says what each became.
///
/// All-or-nothing: every refusal happens before the document is touched, and
/// the result has to validate before it replaces the project.
pub fn set_asset(
    project: &mut Project,
    id: &AssetId,
    edit: &Edit,
) -> Result<Vec<String>, AuthorError> {
    let asked = edit.named();
    if asked.is_empty() {
        return Err(AuthorError::NothingAsked);
    }
    let index = project
        .assets
        .iter()
        .position(|asset| &asset.id == id)
        .ok_or_else(|| AuthorError::NoSuchAsset { asset: id.clone() })?;
    let kind = project.assets[index].kind;
    let takes = fields_of(kind).ok_or_else(|| AuthorError::NotInline {
        asset: id.clone(),
        kind,
    })?;
    if let Some(field) = asked.into_iter().find(|field| !takes.contains(field)) {
        return Err(AuthorError::NotOnKind {
            field,
            kind,
            takes: takes.join(", "),
        });
    }

    let mut proposed = project.clone();
    let changes = apply(&mut proposed.assets[index], edit)?;
    proposed.validate()?;
    *project = proposed;
    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::super::fixture::project;
    use super::*;

    fn edit() -> Edit {
        Edit::default()
    }

    fn styled(project: &Project) -> crate::text::TextStyle {
        project
            .asset(&AssetId::new("caption"))
            .expect("the fixture has a caption")
            .text_style()
    }

    /// **The reason this verb merges.** The caption is in a serif face at
    /// 0.12; asking for a smaller size must not put it back to the default
    /// sans, which is exactly what replacing the style block whole would do
    /// and would say nothing about.
    #[test]
    fn one_field_changes_one_field() {
        let mut project = project();
        let said = set_asset(
            &mut project,
            &AssetId::new("caption"),
            &Edit {
                size: Some(0.08),
                ..edit()
            },
        )
        .expect("a smaller caption");
        assert_eq!(said, vec!["size: 0.12 → 0.08"]);
        let style = styled(&project);
        assert_eq!(style.size, 0.08);
        assert_eq!(style.font.name(), Some("serif"));
    }

    /// Rewording says both halves, because the caller cannot see the document
    /// and *was* is what proves the change landed where it was aimed.
    #[test]
    fn rewording_reports_what_it_replaced() {
        let mut project = project();
        let said = set_asset(
            &mut project,
            &AssetId::new("caption"),
            &Edit {
                text: Some("DUSK".to_owned()),
                ..edit()
            },
        )
        .expect("a reworded caption");
        assert_eq!(said, vec![r#"text: "DAWN" → "DUSK""#]);
    }

    /// The two fields whose value is a *word* rather than a number, so the
    /// line has to name both faces and both edges rather than print an enum.
    #[test]
    fn a_face_and_an_alignment_read_back_as_words() {
        let mut project = project();
        let said = set_asset(
            &mut project,
            &AssetId::new("caption"),
            &Edit {
                font: Some("sans".to_owned()),
                align: Some(TextAlign::Left),
                ..edit()
            },
        )
        .expect("another face, another edge");
        assert_eq!(said, vec!["font: serif → sans", "align: center → left"]);
    }

    /// A caption's rim is set through the same two words a shape's border
    /// takes, and setting it must leave the face and the size alone — the
    /// whole reason this verb merges.
    #[test]
    fn a_caption_takes_a_rim_of_its_own() {
        let mut project = project();
        let said = set_asset(
            &mut project,
            &AssetId::new("caption"),
            &Edit {
                stroke: Some(Rgba::BLACK),
                stroke_width: Some(0.004),
                ..edit()
            },
        )
        .expect("an outlined caption");
        assert_eq!(
            said,
            vec!["stroke: none → #000000", "stroke_width: 0.002 → 0.004"]
        );
        let style = styled(&project);
        assert_eq!(style.stroke, Some(Rgba::BLACK));
        assert_eq!(style.font.name(), Some("serif"));
        assert_eq!(style.size, 0.12);
    }

    #[test]
    fn a_field_of_another_kind_is_refused_and_writes_nothing() {
        let mut project = project();
        let before = project.clone();
        let refused = set_asset(
            &mut project,
            &AssetId::new("caption"),
            &Edit {
                fill: Some(Rgba::BLACK),
                ..edit()
            },
        );
        let problem = refused.expect_err("a caption has no fill");
        assert!(problem.to_string().contains("`fill` is not a field"));
        assert_eq!(project, before);
    }

    /// What a shot *is* lives in a file, and what a brief is lives in a
    /// prompt `rebrief` edits — neither is a field here.
    #[test]
    fn an_asset_with_a_file_behind_it_is_not_this_verb() {
        let mut project = project();
        let refused = set_asset(
            &mut project,
            &AssetId::new("shot"),
            &Edit {
                size: Some(0.2),
                ..edit()
            },
        );
        assert!(matches!(refused, Err(AuthorError::NotInline { .. })));
    }

    #[test]
    fn an_edit_that_names_no_field_is_refused_rather_than_a_quiet_success() {
        let mut project = project();
        let refused = set_asset(&mut project, &AssetId::new("caption"), &edit());
        assert!(matches!(refused, Err(AuthorError::NothingAsked)));
    }

    #[test]
    fn an_asset_that_is_not_there_is_named_in_the_refusal() {
        let mut project = project();
        let refused = set_asset(
            &mut project,
            &AssetId::new("titel"),
            &Edit {
                text: Some("x".to_owned()),
                ..edit()
            },
        );
        assert!(matches!(refused, Err(AuthorError::NoSuchAsset { .. })));
    }

    /// All-or-nothing over validation, not merely over the field checks: a
    /// weight no font could have is refused by [`Project::validate`], and the
    /// caption keeps the size that was asked for in the same call.
    #[test]
    fn a_change_the_document_refuses_leaves_every_other_field_alone() {
        let mut project = project();
        let before = project.clone();
        let refused = set_asset(
            &mut project,
            &AssetId::new("caption"),
            &Edit {
                size: Some(0.05),
                weight: Some(60_000),
                ..edit()
            },
        );
        assert!(matches!(refused, Err(AuthorError::Refused(_))));
        assert_eq!(project, before);
    }
}
