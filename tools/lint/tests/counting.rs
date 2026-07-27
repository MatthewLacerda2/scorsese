//! What counts as a line. `wc -l` counts newlines and so misses a final
//! unterminated line; the gate counts the line anyway, because it is a line
//! someone has to read.

use scorsese_lint::scan::line_count;

fn count(text: &str) -> usize {
    line_count(text.as_bytes())
}

#[test]
fn an_empty_file_has_no_lines() {
    assert_eq!(count(""), 0);
}

#[test]
fn a_trailing_newline_does_not_add_a_line() {
    assert_eq!(count("one\n"), 1);
    assert_eq!(count("one\ntwo\n"), 2);
}

#[test]
fn a_file_that_does_not_end_in_a_newline_still_ends_in_a_line() {
    assert_eq!(count("one"), 1);
    assert_eq!(count("one\ntwo"), 2);
}

#[test]
fn blank_and_comment_lines_count_like_any_other() {
    // The cap is on how much file there is, not on how much of it is code.
    assert_eq!(count("code\n\n// comment\n\n"), 4);
}

#[test]
fn windows_line_endings_count_once_each() {
    assert_eq!(count("one\r\ntwo\r\n"), 2);
}
