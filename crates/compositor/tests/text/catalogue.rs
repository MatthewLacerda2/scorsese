//! The shipped catalogue against what a person reads about it.
//!
//! `docs/project-format.md` is where an agent learns which fonts exist, and
//! a name published there that the code does not ship — or a weight or an
//! italic it does not have — is a refusal the doc walked them into. So the
//! table is parsed and held to [`text::SHIPPED`], row for row.

use scorsese_compositor::text::{self, Cut, Font, FontError, Slant};

const DOC: &str = include_str!("../../../../docs/project-format.md");

/// Every character Portuguese writes that ASCII does not.
const PORTUGUESE: &str = "áàâãéêíóôõúüç ÁÀÂÃÉÊÍÓÔÕÚÜÇ";

/// The rows of the shipped-font table: `| name | family | weights | italic | for |`.
fn rows() -> Vec<Vec<String>> {
    let table = DOC
        .split_once("| name | family | weights | italic | for |")
        .expect("the shipped-font table is in the doc")
        .1;
    table
        .lines()
        .skip(2)
        .take_while(|line| line.starts_with('|'))
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().trim_matches('`').to_owned())
                .collect()
        })
        .collect()
}

/// What the weights cell has to say: a range for a variable file, read from
/// the file's own axis, and the drawn list for a drawn family.
fn weights(cut: Cut) -> String {
    match cut {
        Cut::Variable(bytes) => match Font::from_bytes(bytes, None).err() {
            Some(FontError::VariableWithoutWeight { min, max, .. }) => format!("{min} – {max}"),
            other => panic!("a shipped variable file has a `wght` axis; got {other:?}"),
        },
        Cut::Drawn(files) => files
            .iter()
            .map(|(weight, _)| weight.to_string())
            .collect::<Vec<_>>()
            .join(", "),
    }
}

#[test]
fn the_doc_lists_every_shipped_family_as_it_is() {
    let rows = rows();
    let names: Vec<_> = rows.iter().map(|row| row[0].as_str()).collect();
    let shipped: Vec<_> = text::SHIPPED.iter().map(|family| family.name).collect();
    assert_eq!(
        names, shipped,
        "the doc's table and the catalogue, in order"
    );

    for (row, family) in rows.iter().zip(text::SHIPPED) {
        assert_eq!(row[1], family.family, "`{}`'s family", family.name);
        assert_eq!(row[2], weights(family.cut), "`{}`'s weights", family.name);
        let italic = if family.italic.is_some() {
            "yes"
        } else {
            "—"
        };
        assert_eq!(row[3], italic, "`{}`'s italic", family.name);
    }
}

/// Checked against the files, not the claim: a face that drops the tilde or
/// the cedilla sets "coração" with a hole in it and nothing says so.
#[test]
fn every_shipped_face_sets_portuguese() {
    for family in text::SHIPPED {
        for slant in [Slant::Upright, Slant::Italic] {
            for weight in family.drawn_weights(slant) {
                let font = Font::shipped(family.name, Some(weight), slant).expect("drawn");
                assert_eq!(
                    font.uncovered(PORTUGUESE),
                    Vec::<char>::new(),
                    "{}",
                    family.name
                );
            }
            if let Some(Cut::Variable(_)) = family.table(slant) {
                let font = Font::shipped(family.name, None, slant).expect("resolves");
                assert_eq!(
                    font.uncovered(PORTUGUESE),
                    Vec::<char>::new(),
                    "{}",
                    family.name
                );
            }
        }
    }
}
