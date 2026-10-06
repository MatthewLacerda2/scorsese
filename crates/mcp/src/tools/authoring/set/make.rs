//! Making an asset the id names nothing for yet.

use scorsese_core::{AssetKind, Fill, Icon, Inline, Project, authoring};

use super::super::fill::fill;
use super::super::{maybe, refused, save};
use super::{Arguments, fields, requests, shape, text};

/// Makes the `kind` the arguments describe under `id` (or one derived from
/// its content), writes the document, and says what was made.
pub(super) fn make(
    project: &mut Project,
    dir: &std::path::Path,
    id: Option<&str>,
    kind: AssetKind,
    arguments: &Arguments,
) -> Result<String, String> {
    let (content, after) = match kind {
        AssetKind::Text => (
            text::made(arguments)?,
            "place_clip puts it on a video track, with a duration: a title has no \
             length of its own."
                .to_owned(),
        ),
        AssetKind::Color => {
            let paint = colour(arguments)?;
            let said = format!(
                "{paint}. It fills the frame; place_clip puts it on a video track, with a \
                 duration."
            );
            (Inline::Color(paint), said)
        }
        AssetKind::Shape => {
            let (content, outline) = shape::made(arguments)?;
            (
                content,
                format!("{outline}. place_clip puts it on a video track, with a duration."),
            )
        }
        AssetKind::Icon => (
            icon(arguments)?,
            "project_check says whether that name is one this build ships.".to_owned(),
        ),
        _ => (
            requests::sketch(kind, arguments)?,
            "A sketch: it renders as a slug card, so the cut previews for nothing, and \
             generate realises it — nothing was generated or spent."
                .to_owned(),
        ),
    };
    let id = authoring::add_asset(project, id, content).map_err(refused)?;
    save(project, dir)?;
    Ok(format!(
        "`{id}` — a {} asset, new. {after}",
        fields::kind_name(kind)
    ))
}

/// A colour card. The colour is required and has no default on purpose: a
/// card nobody chose the colour of would render as some colour, and a film
/// that opens on the wrong shade fails silently.
fn colour(arguments: &Arguments) -> Result<Fill, String> {
    // A blank or null colour is somebody not choosing one, which is the one
    // thing this kind refuses — said the way a missing one is.
    fill(arguments.color.as_ref(), "color")?
        .ok_or_else(|| "`color` is required: the colour the card is".to_owned())
}

/// A symbol. Size and colour are both required: a symbol drawn at a size
/// nobody chose, in a colour nobody chose, is a shot that is wrong with
/// nothing to say so.
fn icon(arguments: &Arguments) -> Result<Inline, String> {
    let name =
        maybe(arguments.icon.as_deref()).ok_or("`icon` is required: which symbol to draw")?;
    let size = arguments
        .size
        .ok_or("`size` is required: how big, as a fraction of the frame's height")?;
    let drawn_in = match fill(arguments.color.as_ref(), "color")? {
        Some(paint) => paint.solid().ok_or(
            "`color`: an icon is one colour, as `#rrggbb` — only a color asset takes a gradient",
        )?,
        None => return Err("`color` is required: the colour to draw it in".to_owned()),
    };
    let icon = Icon::new(name, size, drawn_in);
    Ok(Inline::Icon(match arguments.stroke_width {
        Some(width) => icon.weighing(width),
        None => icon,
    }))
}
