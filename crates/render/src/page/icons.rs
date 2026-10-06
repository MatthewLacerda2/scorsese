//! The icons a page is given: the set the `icon` asset draws, as SVG files.
//!
//! Served at `https://lib.scorsese/icons/<name>.svg` (#838), beside the fonts
//! and anime.js, so a page that wants a symbol names one of the seventeen
//! hundred the `icons` search tool finds rather than drawing one from memory.
//! Written on request from the compositor's own contours, so there is one copy
//! of the set in the binary and the page draws what the asset draws.
//!
//! A name the set does not have is not refused like a stranger's host: it is a
//! near miss on a real library, so the page note it becomes names the closest
//! icons, the way validation does for an `icon` asset.

use scorsese_compositor::icon;

/// What the `icons/` folder answers for one file name.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Served {
    /// The icon's document.
    Svg(String),
    /// No icon of that name, and the ones it nearly was, best first.
    Unknown {
        name: String,
        nearest: Vec<&'static str>,
    },
}

/// The answer for `file`, the path under `icons/`.
pub(crate) fn serve(file: &str) -> Served {
    let name = file.strip_suffix(".svg").unwrap_or(file);
    match icon::find(name).and_then(icon::Icon::svg) {
        Some(svg) => Served::Svg(svg),
        None => Served::Unknown {
            name: name.to_owned(),
            nearest: icon::nearest(name),
        },
    }
}

/// The page note for a name the set does not have.
pub(crate) fn unknown(name: &str, nearest: &[&str]) -> String {
    let near = if nearest.is_empty() {
        String::new()
    } else {
        format!(" (nearest: {})", nearest.join(", "))
    };
    format!(
        "the page asked for the icon `{name}`, which scorsese does not ship{near}; \
         the `icons` tool finds names. It rendered without it"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shipped_icon_is_served_as_lucides_own_vocabulary() {
        let Served::Svg(svg) = serve("database.svg") else {
            panic!("database is a vendored icon");
        };
        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""));
        assert!(svg.contains("viewBox=\"0 0 24 24\""));
        assert!(svg.contains("stroke=\"currentColor\""));
        assert!(svg.contains("<path d=\"M"));
    }

    #[test]
    fn a_dot_is_filled_in_the_same_colour() {
        let Served::Svg(svg) = serve("palette.svg") else {
            panic!("palette is a vendored icon");
        };
        assert!(svg.contains("fill=\"currentColor\" stroke=\"none\""));
    }

    #[test]
    fn an_unknown_name_says_what_it_nearly_was() {
        let Served::Unknown { name, nearest } = serve("databse.svg") else {
            panic!("databse is not an icon");
        };
        assert_eq!(name, "databse");
        assert!(nearest.contains(&"database"), "{nearest:?}");
        let note = unknown(&name, &nearest);
        assert!(
            note.contains("`databse`") && note.contains("database"),
            "{note}"
        );
    }
}
