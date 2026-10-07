use crate::{AssetId, Clip, ClipId, Fps, Frames, ProjectPath, Speed};

use super::*;

/// The line the provider was probed with, on `eleven_flash_v2_5`.
fn probed() -> Words {
    let text = "The gradient flows back.";
    let spans = [(0.0, 0.116), (0.151, 0.464), (0.499, 0.708), (0.755, 1.254)];
    let mut characters = Vec::new();
    for (word, (start, end)) in text.split(' ').zip(spans) {
        if !characters.is_empty() {
            characters.push((' ', start, start));
        }
        let each = (end - start) / word.chars().count() as f64;
        for (i, c) in word.chars().enumerate() {
            let at = start + each * i as f64;
            characters.push((c, at, at + each));
        }
    }
    Words::from_characters(characters)
}

#[test]
fn characters_fold_into_words_at_whitespace() {
    let words = probed();
    let said: Vec<_> = words.words.iter().map(|w| w.text.as_str()).collect();
    assert_eq!(said, ["The", "gradient", "flows", "back."]);
    assert!((words.words[1].start - 0.151).abs() < 1e-9);
    assert!((words.words[3].end - 1.254).abs() < 1e-9);
    let spaced = Words::from_characters([(' ', 0.0, 0.1), ('a', 0.1, 0.2), (' ', 0.2, 0.3)]);
    assert_eq!(spaced.words.len(), 1);
}

#[test]
fn a_repeated_word_is_counted_and_punctuation_dropped() {
    let words = Words {
        words: ["Gradient,", "the", "gradient", "—", "don't", "GRADIENT!"]
            .iter()
            .map(|text| Word {
                text: (*text).to_owned(),
                start: 0.0,
                end: 0.0,
            })
            .collect(),
    };
    let names = words.names();
    let names: Vec<_> = names.iter().map(Option::as_deref).collect();
    assert_eq!(
        names,
        [
            Some("gradient"),
            Some("the"),
            Some("gradient@2"),
            None,
            Some("don't"),
            Some("gradient@3")
        ]
    );
}

#[test]
fn the_timings_sit_beside_the_audio() {
    let audio = ProjectPath::new("generated/nar-9-abc.mp3");
    assert_eq!(
        Words::beside(&audio).as_str(),
        "generated/nar-9-abc.words.json"
    );
    let back: Words = serde_json::from_str(&probed().to_json()).unwrap();
    assert_eq!(back, probed());
}

#[test]
fn words_land_where_the_clip_plays_them() {
    let fps = Fps::THIRTY;
    let mut clip = Clip::new(
        ClipId::new("nar-9"),
        AssetId::new("vo"),
        Frames(300),
        Frames(30),
    );
    // Starting at 10 s, playing from 0.5 s into the line at double speed:
    // 0.5–2.5 s of the audio is heard over 10–11 s.
    clip.source_in = Frames(15);
    clip.speed = Speed::new(2.0);
    let placed = probed().placed(&clip, fps);
    let names: Vec<_> = placed.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        names,
        ["flows", "back"],
        "`The` and `gradient` are trimmed off"
    );
    assert!((placed[0].start - (10.0 + (0.499 - 0.5) / 2.0)).abs() < 1e-9);
    assert!((placed[1].end - (10.0 + (1.254 - 0.5) / 2.0)).abs() < 1e-9);
    assert_eq!(placed[1].text, "back.");
}
