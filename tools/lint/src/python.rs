//! Lines of code in a Python file, under the same rule as Rust's.
//!
//! [`crate::scan::code_lines`] has the argument: what grows when a file does
//! too much is its code, and this repo documents at length on purpose, so prose
//! is free. Python's prose is `#` comments and docstrings, and the scripts in
//! `.github/scripts/` carry a great deal of both — `merge-queue.py`'s module
//! doc alone is a page and a half (#1136).

/// A triple-quoted string open across lines, and whether it is prose.
#[derive(Clone, Copy)]
enum Open {
    /// Nothing is open: the next line is read from its start.
    Nothing,
    /// A string that started a line, which is a docstring or reads like one.
    Prose(&'static [u8]),
    /// A string that opened after code, such as `X = """`: its lines are data
    /// the code holds, so they count.
    Data(&'static [u8]),
}

const DELIMITERS: [&[u8]; 2] = [b"\"\"\"", b"'''"];

/// Lines of code in a Python file's bytes: every line that is not blank, not a
/// `#` comment, and not inside a docstring.
///
/// A docstring is any triple-quoted string that *starts a line*, which is every
/// docstring there is and, rarely, a long string argument on a line of its own.
/// The second is prose in all but grammar, so counting it as one costs nothing.
/// A triple quote opened after code (`HELP = """`) is followed to its close,
/// so a line inside it that happens to start `"""` or `#` is still data, never
/// mistaken for the start of a docstring or a comment.
///
/// Quotes are found by counting, not by tokenizing: a triple quote inside an
/// ordinary string or a trailing comment would be miscounted. The scripts have
/// none, and a gate that needs a Python parser to run is a gate with a
/// dependency.
pub fn code_lines(bytes: &[u8]) -> usize {
    let mut open = Open::Nothing;
    bytes
        .split(|&b| b == b'\n')
        .filter(|line| is_code(line.trim_ascii(), &mut open))
        .count()
}

/// Whether one trimmed line counts, given what is open as it starts — and
/// noting what is open as it ends.
fn is_code(line: &[u8], open: &mut Open) -> bool {
    match *open {
        Open::Prose(quote) => {
            let Some(end) = find(line, quote) else {
                return false;
            };
            *open = Open::Nothing;
            is_code(&line[end + quote.len()..], open)
        }
        Open::Data(quote) => {
            if occurrences(line, quote) % 2 == 1 {
                *open = Open::Nothing;
            }
            !line.is_empty()
        }
        Open::Nothing if line.is_empty() || line.starts_with(b"#") => false,
        Open::Nothing => {
            if let Some((quote, rest)) = opens_prose(line) {
                *open = Open::Prose(quote);
                return is_code(rest, open);
            }
            if let Some(quote) = DELIMITERS
                .into_iter()
                .find(|quote| occurrences(line, quote) % 2 == 1)
            {
                *open = Open::Data(quote);
            }
            true
        }
    }
}

/// The delimiter and what follows it, if `line` starts with a triple-quoted
/// string — after any of the prefixes Python allows on one (`r"""`, `b'''`…).
fn opens_prose(line: &[u8]) -> Option<(&'static [u8], &[u8])> {
    let unprefixed = line
        .iter()
        .take(2)
        .take_while(|b| b"rRbBuUfF".contains(b))
        .count();
    let body = &line[unprefixed..];
    DELIMITERS
        .into_iter()
        .find(|quote| body.starts_with(quote))
        .map(|quote| (quote, &body[quote.len()..]))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

/// How many times `needle` appears in `haystack`, without overlaps — so
/// `""""""`, an empty triple-quoted string, is two and closes itself.
fn occurrences(haystack: &[u8], needle: &[u8]) -> usize {
    let mut count = 0;
    let mut rest = haystack;
    while let Some(at) = find(rest, needle) {
        count += 1;
        rest = &rest[at + needle.len()..];
    }
    count
}
