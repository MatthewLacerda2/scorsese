//! A skill's frontmatter, held to what Claude requires to load it.
//!
//! `SKILL.md` opens with a `---` block whose `name` is at most
//! [`NAME_LIMIT`] characters of lower-case letters, digits and hyphens, equal
//! to its folder's name, and whose `description` is not empty and at most
//! [`DESCRIPTION_LIMIT`] characters. A skill outside these is not loaded, and
//! nothing says so at the time; this says so at commit.

/// The rule's name, as an `allow` names it.
pub const RULE: &str = "skill";

/// The longest a skill's name may be, in characters.
pub const NAME_LIMIT: usize = 64;

/// The longest a skill's description may be, in characters.
pub const DESCRIPTION_LIMIT: usize = 1024;

/// The value of top-level `key` in a frontmatter block, its text joined by
/// spaces: a plain scalar with any indented lines that continue it (the text
/// may start on the next line), a quoted one, or a `>` / `|` block scalar
/// with any chomping or indentation indicator, blank lines included. A plain
/// scalar's ` #` comment is not part of it.
pub fn field(block: &[&str], key: &str) -> Option<String> {
    let index = block.iter().position(|line| {
        line.strip_prefix(key)
            .is_some_and(|rest| rest.starts_with(':'))
    })?;
    let first = block[index][key.len() + 1..].trim();
    let rest: Vec<&str> = block[index + 1..]
        .iter()
        .take_while(|line| line.trim().is_empty() || line.starts_with([' ', '\t']))
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    if is_block_indicator(first) {
        return Some(rest.join(" "));
    }
    let quoted = ['"', '\'']
        .iter()
        .find_map(|quote| first.strip_prefix(*quote)?.strip_suffix(*quote));
    if let Some(quoted) = quoted {
        return Some(quoted.to_string());
    }
    let plain = first
        .split_once(" #")
        .map_or(first, |(value, _)| value)
        .trim();
    Some(
        std::iter::once(plain)
            .chain(rest)
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Whether a value opens a block scalar: `>` or `|`, then at most a chomping
/// indicator (`+`, `-`) and an indentation digit, in either order.
fn is_block_indicator(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('>' | '|'))
        && chars.as_str().len() <= 2
        && chars.all(|c| matches!(c, '+' | '-') || c.is_ascii_digit())
}

/// The lines between the opening `---` and the closing one, if the text
/// opens with a frontmatter block.
pub fn block(text: &str) -> Option<Vec<&str>> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    let inside: Vec<&str> = lines
        .by_ref()
        .take_while(|line| line.trim_end() != "---")
        .collect();
    // `take_while` ate the closing line if there was one; an unclosed block
    // reads to the end, which is no block at all.
    (inside.len() < text.lines().count() - 1).then_some(inside)
}

/// What is wrong with the frontmatter of a `SKILL.md` in `folder`, one
/// message each.
pub fn frontmatter_findings(text: &str, folder: &str) -> Vec<String> {
    let Some(block) = block(text) else {
        return vec![
            "no frontmatter: open with a `---` block holding `name` and `description`".into(),
        ];
    };
    let mut wrong = Vec::new();
    match field(&block, "name") {
        None => wrong.push(format!("no `name`: add `name: {folder}`")),
        Some(name) => {
            let length = name.chars().count();
            if name.is_empty() {
                wrong.push(format!("an empty `name`: write `name: {folder}`"));
            } else if length > NAME_LIMIT {
                wrong.push(format!(
                    "a `name` of {length} characters, over {NAME_LIMIT}"
                ));
            }
            if !name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
            {
                wrong.push(format!(
                    "`name: {name}` may hold only lower-case letters, digits and hyphens"
                ));
            }
            if !name.is_empty() && name != folder {
                wrong.push(format!(
                    "`name: {name}` differs from its folder, `{folder}`: rename one"
                ));
            }
        }
    }
    match field(&block, "description") {
        None => wrong.push("no `description`: say what the skill does and when to use it".into()),
        Some(description) if description.trim().is_empty() => {
            wrong.push("an empty `description`: say what the skill does and when to use it".into());
        }
        Some(description) => {
            let length = description.chars().count();
            if length > DESCRIPTION_LIMIT {
                wrong.push(format!(
                    "a `description` of {length} characters, over {DESCRIPTION_LIMIT}: say when \
                     to use the skill and leave how to the body"
                ));
            }
        }
    }
    wrong
}
