//! The faces a page is given: scorsese's own, and nothing of the host's.
//!
//! Every shipped face is declared to the page under its family's name before
//! the page's scripts run — `font-family: Inter` or `"Playfair Display"` means
//! the file this build ships, served from [`super::SHIPPED_ORIGIN`], exactly
//! as a text asset naming it would get. Declared by name rather than left to
//! the system's font matching because the files' own names do not agree with
//! the catalogue's (`Inter-V.ttf` calls itself *Inter V*), and the catalogue is
//! what an author reads.
//!
//! The same files are also the **only** ones the browser's fallback can reach
//! — written out with a fontconfig file listing just them ([`super::cache`]) —
//! so a page naming a font nobody gave it lands on the same face on every
//! machine, never on whatever the host installed (#606).

use scorsese_compositor::text::{Cut, SHIPPED};

/// One shipped file, and what it is.
pub(crate) struct Face {
    /// The name it is served and written under.
    pub(crate) file: String,
    /// The family, as the catalogue spells it.
    pub(crate) family: &'static str,
    /// The scorsese name of the family (`inter`), also declared.
    pub(crate) name: &'static str,
    /// The weight it was drawn at, or `None` for a variable file.
    pub(crate) weight: Option<u16>,
    pub(crate) italic: bool,
    pub(crate) bytes: &'static [u8],
}

/// Every shipped face file.
pub(crate) fn faces() -> Vec<Face> {
    let mut faces = Vec::new();
    for family in SHIPPED {
        for (italic, cut) in [(false, Some(family.cut)), (true, family.italic)] {
            let slant = if italic { "-italic" } else { "" };
            let face = |file: String, weight, bytes| Face {
                file,
                family: family.family,
                name: family.name,
                weight,
                italic,
                bytes,
            };
            match cut {
                Some(Cut::Variable(bytes)) => {
                    faces.push(face(format!("{}{slant}.ttf", family.name), None, bytes));
                }
                Some(Cut::Drawn(weights)) => faces.extend(weights.iter().map(|(weight, bytes)| {
                    face(
                        format!("{}-{weight}{slant}.ttf", family.name),
                        Some(*weight),
                        *bytes,
                    )
                })),
                None => {}
            }
        }
    }
    faces
}

/// The bytes of the shipped face served as `file`.
pub(crate) fn bytes(file: &str) -> Option<&'static [u8]> {
    faces()
        .into_iter()
        .find(|face| face.file == file)
        .map(|face| face.bytes)
}

/// The script declaring every face to the page, by family and by scorsese name.
pub(crate) fn declarations(origin: &str) -> String {
    let mut script = String::from(
        "(() => {\n  const add = (family, url, weight, style) =>\n    document.fonts.add(new FontFace(family, `url(${url})`, { weight, style }));\n",
    );
    for face in faces() {
        let url = format!("{origin}/fonts/{}", face.file);
        let weight = face
            .weight
            .map_or_else(|| "1 1000".to_owned(), |w| w.to_string());
        let style = if face.italic { "italic" } else { "normal" };
        let mut names = vec![face.family];
        if !face.name.eq_ignore_ascii_case(face.family) {
            names.push(face.name);
        }
        for name in names {
            script.push_str(&format!(
                "  add({name:?}, {url:?}, {weight:?}, {style:?});\n"
            ));
        }
    }
    script.push_str("})();\n");
    script
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shipped_face_has_a_file_name_of_its_own() {
        let mut names: Vec<_> = faces().into_iter().map(|face| face.file).collect();
        let count = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), count);
        assert!(names.contains(&"inter.ttf".to_owned()));
        assert!(names.contains(&"liberation-sans-700-italic.ttf".to_owned()));
        assert!(bytes("inter.ttf").is_some_and(|b| !b.is_empty()));
        assert!(bytes("nope.ttf").is_none());
    }

    #[test]
    fn faces_are_declared_by_the_catalogue_name_not_the_file_name() {
        let script = declarations("https://lib.scorsese");
        assert!(script.contains(
            r#"add("Inter", "https://lib.scorsese/fonts/inter.ttf", "1 1000", "normal");"#
        ));
        assert!(script.contains(r#"add("Liberation Sans", "https://lib.scorsese/fonts/liberation-sans-700.ttf", "700", "normal");"#));
        assert!(
            script.contains(r#"add("liberation-sans", "#),
            "the scorsese name too"
        );
        assert!(
            !script.contains(r#"add("inter", "#),
            "a name that differs only in case is the same family"
        );
    }
}
