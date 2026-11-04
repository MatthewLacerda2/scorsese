use scorsese_core::captions::{Chunk, Chunking, chunks};
use scorsese_core::words::Placed;
use scorsese_core::{ClipId, FontChoice, ProjectPath, TrackId};

use super::super::Choices;
use super::{html, quoted};

fn said(line: &str, text: &str) -> Vec<Chunk> {
    let words: Vec<Placed> = text
        .split(' ')
        .enumerate()
        .map(|(i, word)| Placed {
            name: word.to_lowercase(),
            text: word.to_owned(),
            start: i as f64 * 0.3,
            end: i as f64 * 0.3 + 0.3,
        })
        .collect();
    chunks(&ClipId::new(line), &words, Chunking::WORDS)
}

fn page(choices: Choices) -> String {
    let asked = choices.captioning(TrackId::new("captions"), 30.0);
    html(&asked, &said("vo", "Hello there"))
}

#[test]
fn every_part_of_the_template_is_filled() {
    let page = page(Choices::default());
    assert!(!page.contains("{{"), "{page}");
    assert!(
        page.contains(r#"[[["vo/hello","Hello"],["vo/there","there"]]]"#),
        "{page}"
    );
    assert!(page.contains("font-family: \"montserrat\""), "{page}");
    assert!(page.contains("color: #ffd400"), "the lit word is yellow");
    assert!(
        page.contains("-webkit-text-stroke: 0.72vh #000000"),
        "{page}"
    );
    assert!(
        page.contains("const LEAD = 0.07, HOLD = 0.35, BRIDGE = 1;"),
        "{page}"
    );
    for part in [
        "font-size: 3.8vh",
        "bottom: 20%",
        "font-weight: 900",
        "color: #ffffff",
    ] {
        assert!(page.contains(part), "{part} in {page}");
    }
}

#[test]
fn a_word_cannot_close_the_script() {
    assert_eq!(quoted("a</script>\"\\"), r#""a\u003c/script>\"\\""#);
    assert_eq!(quoted("line\nbreak"), r#""line\u000abreak""#);
    assert_eq!(
        quoted("a b\u{2028}"),
        r#""a b\u2028""#,
        "a space is a space"
    );
}

#[test]
fn no_rim_and_a_project_font_are_honoured() {
    let page = page(Choices {
        stroke: Some(super::super::Stroke(None)),
        font: Some(FontChoice::File(ProjectPath::new("assets/Face[wght].ttf"))),
        ..Choices::default()
    });
    assert!(
        page.contains("-webkit-text-stroke: 0vh transparent"),
        "{page}"
    );
    assert!(
        page.contains(r#"src: url("../assets/Face%5Bwght%5D.ttf")"#),
        "{page}"
    );
    assert!(
        page.contains("font-weight: normal"),
        "a file is its own weight"
    );
}

#[test]
fn the_text_look_is_one_choice_away() {
    let asked = Choices {
        text: true,
        ..Choices::default()
    }
    .captioning(TrackId::new("captions"), 30.0);
    assert!(matches!(asked.look, scorsese_core::captions::Look::Text));
    assert_eq!(asked.chunking, Chunking::DEFAULT);
}
