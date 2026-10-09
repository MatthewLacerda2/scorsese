use super::outline::{self, LIMIT};
use super::{GUIDES, read};

/// No answer is longer than the limit — the whole of a short guide, the map of
/// a long one, and every section of every guide, at every depth.
#[test]
fn every_answer_is_bounded() {
    for guide in GUIDES {
        let whole = read(guide.name, None).unwrap();
        assert!(
            whole.len() <= LIMIT,
            "guide {} is {} bytes",
            guide.name,
            whole.len()
        );
        for number in 1..=outline::headings(guide.text).len() {
            let part = read(guide.name, Some(&number.to_string())).unwrap();
            assert!(
                part.len() <= LIMIT,
                "guide {} section {number} is {} bytes",
                guide.name,
                part.len()
            );
        }
    }
}

#[test]
fn a_short_guide_comes_back_exactly_as_the_file_has_it() {
    let prompts = GUIDES.iter().find(|guide| guide.name == "prompts").unwrap();
    assert_eq!(read("prompts", None).unwrap(), prompts.text);
}

#[test]
fn a_long_guide_comes_back_as_its_opening_and_its_sections() {
    let map = read("recipes", None).unwrap();
    assert!(map.starts_with("# "));
    assert!(map.contains("too long to hand back whole"));
    assert!(map.contains(". Music: `\"recipe\": \"song\"` (about"));
}

#[test]
fn a_section_is_found_by_its_words_whatever_their_markup_or_case() {
    let part = read("pages", Some("a lottie ANIMATION")).unwrap();
    assert!(part.starts_with("### A Lottie animation\n"), "{part}");
    let crop = read("project-format", Some("showing part of a source: crop")).unwrap();
    assert!(crop.starts_with("### Showing part of a source: `crop`"));
    let anchored = read("recipes", Some("#starting-from-a-midi-file")).unwrap();
    assert!(anchored.starts_with("### Starting from a MIDI file"));
}

#[test]
fn a_section_runs_to_the_next_heading_at_its_level() {
    let text = "# T\n\n## One\na\n### One.a\nb\n## Two\nc\n";
    let headings = outline::headings(text);
    assert_eq!(
        &text[headings[0].start..headings[0].end],
        "## One\na\n### One.a\nb\n"
    );
    assert_eq!(&text[headings[1].start..headings[1].end], "### One.a\nb\n");
    assert_eq!(&text[headings[2].start..headings[2].end], "## Two\nc\n");
}

#[test]
fn a_hash_inside_a_code_block_is_not_a_heading() {
    let text = "# T\n\n## Real\n```sh\n## not a heading\n```\n";
    let titles: Vec<&str> = outline::headings(text).iter().map(|h| h.title).collect();
    assert_eq!(titles, ["Real"]);
}

#[test]
fn words_that_name_two_sections_are_answered_with_both_numbers() {
    let refused = read("project-format", Some("The combinations that are refused")).unwrap_err();
    assert!(refused.contains("names 2 sections"), "{refused}");
    assert!(
        refused.contains("(in What a generated video asks for)"),
        "{refused}"
    );
}

#[test]
fn what_is_not_there_is_refused_with_what_is() {
    let refused = read("tutorial", None).unwrap_err();
    assert!(refused.contains("pages, project-format"), "{refused}");
    let missing = read("pages", Some("nothing like this")).unwrap_err();
    assert!(missing.contains("The contract"), "{missing}");
    assert!(read("pages", Some("999")).is_err());
}

#[test]
fn sizes_are_said_in_tokens() {
    assert_eq!(outline::about(330), "about 100 tokens");
    assert_eq!(outline::about(10), "about 100 tokens");
    assert_eq!(outline::about(33_000), "about 10.0k tokens");
}
