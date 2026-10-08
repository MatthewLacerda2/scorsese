//! A guide cut at its headings, and the part of it a call asked for.
//!
//! Markdown's own structure is the index: a `##` heading opens a section that
//! runs to the next heading at its level or above, so a section is found by
//! its words and handed back exactly as the page has it — never paraphrased,
//! never a copy that could drift from the file.
//!
//! **A part too long to hand back whole is handed back as a map instead**: its
//! opening, then every heading inside it, numbered and sized, so the next call
//! asks for the one it needs. The same rule applies at every depth, which is
//! what keeps the answer bounded whichever guide, or whichever section of one,
//! was asked for.

/// The longest part handed back whole, in bytes: 32 KiB, about ten thousand
/// tokens.
///
/// Measured on 2026-10-08 (#909) with a tokenizer's own count: `pages` is
/// 28 KB and 8.7k tokens, and it is the guide read most — before every page —
/// so it comes back whole in one call. `recipes` is 143 KB (42k tokens) and
/// `project-format` 184 KB, which no call should hand back whole: an answer
/// stays in the conversation, and the web assistant pays for it again on
/// every later model call of the turn. Every section of either is under the
/// limit once its subsections are listed rather than inlined.
pub(super) const LIMIT: usize = 32 * 1024;

/// One heading, and the stretch of the page it opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Heading<'a> {
    /// 2 for `##`, 3 for `###`, and so on. The page's own `#` title is the
    /// guide itself, not a section of it.
    pub(super) level: usize,
    /// The heading's words, as written.
    pub(super) title: &'a str,
    /// Where its line starts.
    pub(super) start: usize,
    /// Where its section ends: the next heading at its level or above, or the
    /// end of the page.
    pub(super) end: usize,
}

/// Every heading below the title, in page order — skipping anything inside a
/// fenced code block, where a `#` is a comment in an example, not a heading.
pub(super) fn headings(text: &str) -> Vec<Heading<'_>> {
    let mut found: Vec<Heading<'_>> = Vec::new();
    let mut fenced = false;
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        let start = at;
        at += line.len();
        let trimmed = line.trim_end();
        if trimmed.trim_start().starts_with("```") || trimmed.trim_start().starts_with("~~~") {
            fenced = !fenced;
            continue;
        }
        if fenced {
            continue;
        }
        let level = trimmed.bytes().take_while(|byte| *byte == b'#').count();
        let Some(title) = trimmed[level..].strip_prefix(' ') else {
            continue;
        };
        if level >= 2 {
            found.push(Heading {
                level,
                title: title.trim(),
                start,
                end: text.len(),
            });
        }
    }
    for index in 0..found.len() {
        let level = found[index].level;
        if let Some(next) = found[index + 1..].iter().find(|next| next.level <= level) {
            found[index].end = next.start;
        }
    }
    found
}

/// A size as an agent budgets it: tokens, roughly. Three and a third bytes a
/// token is what the guides measured (#909); it is said as "about", because
/// every model counts its own way.
pub(super) fn about(bytes: usize) -> String {
    let tokens = bytes * 10 / 33;
    if tokens < 1000 {
        format!("about {} tokens", ((tokens + 50) / 100).max(1) * 100)
    } else {
        format!("about {:.1}k tokens", tokens as f64 / 1000.0)
    }
}

/// The part `start..end` of `text` (whole when it fits), or its opening and a
/// numbered map of the headings inside it when it does not.
///
/// `named` is what the part is called in the note under the map: the guide's
/// name, or a section's heading.
pub(super) fn part(
    text: &str,
    all: &[Heading<'_>],
    start: usize,
    end: usize,
    named: &str,
) -> String {
    let whole = &text[start..end];
    if whole.len() <= LIMIT {
        return whole.to_owned();
    }
    let inside: Vec<(usize, &Heading<'_>)> = all
        .iter()
        .enumerate()
        .filter(|(_, heading)| heading.start > start && heading.start < end)
        .collect();
    let opening_end = inside.first().map_or(end, |(_, first)| first.start);
    let mut said = text[start..opening_end].trim_end().to_owned();
    said.push_str(&format!(
        "\n\n---\n{named} is {} — too long to hand back whole, so this is its opening \
         and the headings inside it. Call guide again with one of them as `section`, \
         by its words or its number:\n",
        about(whole.len())
    ));
    let top = inside
        .iter()
        .map(|(_, heading)| heading.level)
        .min()
        .unwrap_or(2);
    for (index, heading) in inside {
        said.push_str(&format!(
            "\n{}{}. {} ({})",
            "  ".repeat(heading.level - top),
            index + 1,
            heading.title,
            about(heading.end - heading.start)
        ));
    }
    said
}

/// A heading's words with everything but letters and digits taken out and the
/// case folded, so `` `crop` ``, *crop* and CROP are one name — and so is the
/// anchor a page links a heading by (`#starting-from-a-midi-file`), which is
/// how the guides point at their own sections.
fn plain(title: &str) -> String {
    title
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The heading `asked` names: its number in a map, its words exactly, or the
/// only heading whose words contain it. The refusal lists what there is.
pub(super) fn find<'a, 'h>(all: &'h [Heading<'a>], asked: &str) -> Result<&'h Heading<'a>, String> {
    if let Ok(number) = asked.trim().trim_end_matches('.').parse::<usize>() {
        return number
            .checked_sub(1)
            .and_then(|index| all.get(index))
            .ok_or_else(|| {
                format!(
                    "there is no section {number}: they run from 1 to {}",
                    all.len()
                )
            });
    }
    let asked = plain(asked);
    let exact: Vec<usize> = (0..all.len())
        .filter(|i| plain(all[*i].title) == asked)
        .collect();
    let near: Vec<usize> = if exact.is_empty() {
        (0..all.len())
            .filter(|i| plain(all[*i].title).contains(&asked))
            .collect()
    } else {
        exact
    };
    match near.as_slice() {
        [only] => Ok(&all[*only]),
        [] => Err(format!(
            "no heading here says \"{asked}\"; the sections are:{}",
            listed(all, 0..all.len())
        )),
        several => Err(format!(
            "\"{asked}\" names {} sections; pass one's number instead:{}",
            several.len(),
            listed(all, several.iter().copied())
        )),
    }
}

/// Headings as numbered lines, for a refusal to list — each beside the
/// section it sits in, because two headings can share their words (a page
/// asks "the combinations that are refused" of more than one asset kind).
fn listed(all: &[Heading<'_>], which: impl Iterator<Item = usize>) -> String {
    which
        .map(|index| {
            let heading = &all[index];
            let under = all[..index]
                .iter()
                .rev()
                .find(|above| above.level < heading.level)
                .map_or_else(String::new, |above| format!(" (in {})", above.title));
            format!("\n{}. {}{under}", index + 1, heading.title)
        })
        .collect()
}
