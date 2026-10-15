//! The served guides point at each other the way every surface can follow
//! (#936): `guide <name>`, never only a path into a checkout.

use super::{GUIDES, names, read};

/// The paragraphs of a guide: runs of lines between blank ones.
fn paragraphs(text: &str) -> impl Iterator<Item = &str> {
    text.split("\n\n")
}

#[test]
fn no_guide_names_another_by_its_docs_path() {
    for guide in GUIDES {
        for name in names() {
            let path = format!("docs/{name}.md");
            assert!(
                !guide.text.contains(&path),
                "guide {} names `{path}`; write `guide {name}`",
                guide.name
            );
        }
    }
}

/// A Markdown link to another guide's file is for the repository reader;
/// through the tool it is a dead end, so the paragraph also names the guide.
#[test]
fn every_link_to_a_guide_has_the_guide_beside_it() {
    for guide in GUIDES {
        for paragraph in paragraphs(guide.text) {
            for name in names() {
                let linked = format!("]({name}.md)");
                let anchored = format!("]({name}.md#");
                if paragraph.contains(&linked) || paragraph.contains(&anchored) {
                    assert!(
                        paragraph.contains(&format!("`guide {name}`")),
                        "guide {} links {name}.md without `guide {name}` beside it:\n{paragraph}",
                        guide.name
                    );
                }
            }
        }
    }
}

/// `` `guide <name>`, section "<words>" `` is an instruction a reader will
/// carry out, so the words must find exactly one section.
#[test]
fn every_named_section_is_one_the_guide_finds() {
    for guide in GUIDES {
        let flat = guide.text.replace('\n', " ");
        for name in names() {
            let pointer = format!("`guide {name}`, section \"");
            for (at, _) in flat.match_indices(&pointer) {
                let rest = &flat[at + pointer.len()..];
                let words = &rest[..rest.find('"').expect("a closing quote")];
                let found = read(name, Some(words));
                assert!(
                    found.is_ok(),
                    "guide {} points at `guide {name}`, section \"{words}\": {found:?}",
                    guide.name
                );
            }
        }
    }
}
