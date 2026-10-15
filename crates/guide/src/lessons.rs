//! The prompt lessons in a line each, for the tools that send a prompt (#1309).
//!
//! `docs/prompts.md` opens every paid lesson with `**In a line:**` and a
//! sentence, under one heading per provider, and closes the vendor sections
//! the tools carry with an `**In a line each**` list. Those sentences are read
//! out of the page here, so a tool's description and the guide cannot say two
//! different things: there is no second copy to forget. The reasons, the
//! incidents and the rest of the vendors' advice stay in the guide.

use crate::outline;
#[cfg(test)]
use crate::outline::Heading;

/// The brief blocks a lesson can be about: a shot's `video`, a still's
/// `image`, a line's `speech`.
pub const BLOCKS: [&str; 3] = ["video", "image", "speech"];

/// Each section of the page whose lines ride on a block, and that block.
const SECTIONS: [(&str, &str); 5] = [
    ("Veo, learned by paying", "video"),
    ("Nano Banana, learned by paying", "image"),
    ("ElevenLabs, learned by paying", "speech"),
    ("Veo, in Google's words", "video"),
    ("Nano Banana, in Google's words", "image"),
];

/// Where the paid half ends and the vendors' advice begins.
#[cfg(test)]
const ADVICE: &str = "What the vendors advise";

/// What opens a paid lesson's one line.
const ONE: &str = "**In a line:**";

/// What opens a vendor section's list of them.
const EACH: &str = "**In a line each**";

/// The lessons about `block`, a sentence each, in the page's order: the paid
/// ones first, then the vendor's. Empty for a block nobody wrote any for.
pub fn lessons(block: &str) -> Vec<String> {
    lessons_in(crate::text_of("prompts"), block)
}

/// [`lessons`], over any page shaped like `docs/prompts.md`.
pub(crate) fn lessons_in(text: &str, block: &str) -> Vec<String> {
    let headings = outline::headings(text);
    let mut found = Vec::new();
    for (title, tagged) in SECTIONS {
        if tagged != block {
            continue;
        }
        if let Some(section) = headings.iter().find(|heading| heading.title == title) {
            found.extend(lines(&text[section.start..section.end]));
        }
    }
    found
}

/// Every one-line summary and every listed line in `part`, as one sentence
/// each: a summary runs to the end of its paragraph, a listed line to the
/// next item or the end of the list.
fn lines(part: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut paragraphs = part.split("\n\n").map(str::trim);
    while let Some(paragraph) = paragraphs.next() {
        if let Some(rest) = paragraph.strip_prefix(ONE) {
            found.push(joined(rest));
        } else if paragraph.starts_with(EACH) {
            let Some(list) = paragraphs.next() else {
                break;
            };
            let mut item: Option<String> = None;
            for line in list.lines() {
                if let Some(start) = line.strip_prefix("- ") {
                    found.extend(item.take());
                    item = Some(start.trim().to_owned());
                } else if let Some(open) = item.as_mut() {
                    open.push(' ');
                    open.push_str(line.trim());
                }
            }
            found.extend(item);
        }
    }
    found
}

/// A paragraph's lines as one sentence, with the line breaks Markdown ignores
/// taken out and its first letter a capital, as a listed line's already is.
fn joined(paragraph: &str) -> String {
    let words = paragraph.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut letters = words.chars();
    letters.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(letters).collect()
    })
}

/// What is wrong with how `text` marks its lessons, a sentence each: a
/// heading in the paid half that is no provider's, a paid lesson without
/// exactly one line, a line with a link in it (a tool's description has
/// nowhere to follow one), a block nothing is written for.
#[cfg(test)]
fn problems(text: &str) -> Vec<String> {
    let headings = outline::headings(text);
    let mut problems = Vec::new();
    let paid: Vec<&Heading<'_>> = headings
        .iter()
        .skip_while(|heading| heading.level != 2)
        .take_while(|heading| heading.title != ADVICE)
        .collect();
    for heading in paid.iter().filter(|heading| heading.level == 2) {
        if !SECTIONS.iter().any(|(title, _)| *title == heading.title) {
            problems.push(format!(
                "\"{}\" is in the paid half and is no provider's: a lesson sits \
                 under \"<provider>, learned by paying\"",
                heading.title
            ));
        }
    }
    for lesson in paid.iter().filter(|heading| heading.level == 3) {
        let summaries = lines(&text[lesson.start..lesson.end]).len();
        if summaries != 1 {
            problems.push(format!(
                "the lesson \"{}\" has {summaries} lines opening `{ONE}`, and needs one",
                lesson.title
            ));
        }
    }
    for block in BLOCKS {
        let found = lessons_in(text, block);
        if found.is_empty() {
            problems.push(format!("nothing is written for `{block}`"));
        }
        for line in found.iter().filter(|line| line.contains("](")) {
            problems.push(format!("a `{block}` line links somewhere: {line}"));
        }
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The page itself is marked the way the tools read it.
    #[test]
    fn the_prompts_page_marks_every_lesson() {
        let problems = problems(crate::text_of("prompts"));
        assert!(problems.is_empty(), "{problems:#?}");
    }

    const PAGE: &str = "# P\n\nOpening.\n\n## Veo, learned by paying\n\n\
        ### Slow is bought\n\n**In a line:** ask for slow\nmotion in words.\n\n\
        Why, at length.\n\n## Nano Banana, learned by paying\n\n### Name it\n\n\
        **In a line:** name the medium.\n\n## ElevenLabs, learned by paying\n\n\
        ### Model\n\n**In a line:** pick expressive.\n\n## What the vendors advise\n\n\
        ### Veo, in Google's words\n\nAdvice.\n\n**In a line each**, as carried:\n\n\
        - Say the subject,\n  then the place.\n- Write in English.\n\nMore advice.\n";

    #[test]
    fn a_well_marked_page_passes() {
        assert_eq!(problems(PAGE), Vec::<String>::new());
    }

    #[test]
    fn each_block_gets_its_own_lines_paid_first() {
        assert_eq!(
            lessons_in(PAGE, "video"),
            [
                "Ask for slow motion in words.",
                "Say the subject, then the place.",
                "Write in English."
            ]
        );
        assert_eq!(lessons_in(PAGE, "speech"), ["Pick expressive."]);
    }

    #[test]
    fn a_lesson_without_its_line_is_flagged() {
        let page = PAGE.replace("**In a line:** name the medium.", "Name the medium.");
        let problems = problems(&page);
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("\"Name it\" has 0 lines")),
            "{problems:#?}"
        );
    }

    #[test]
    fn a_lesson_under_no_provider_is_flagged() {
        let page = PAGE.replace("## Nano Banana, learned by paying", "## Stills");
        let problems = problems(&page);
        assert!(
            problems
                .iter()
                .any(|problem| problem.contains("\"Stills\" is in the paid half")),
            "{problems:#?}"
        );
    }

    #[test]
    fn a_line_with_a_link_is_flagged() {
        let page = PAGE.replace("pick expressive.", "see [this](#model).");
        assert!(
            problems(&page)
                .iter()
                .any(|problem| problem.contains("links somewhere"))
        );
    }
}
