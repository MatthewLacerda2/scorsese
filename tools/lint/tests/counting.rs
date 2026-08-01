//! What counts as a line of code. Two departures from `wc -l`, both
//! deliberate: a final unterminated line is still a line someone has to read,
//! and blank and comment lines are not code, so they do not count at all.

use scorsese_lint::scan::code_lines;

fn count(text: &str) -> usize {
    code_lines(text.as_bytes())
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
fn blank_lines_do_not_count() {
    assert_eq!(count("one\n\n\ntwo\n"), 2);
    assert_eq!(count("\n\n   \n\t\n"), 0);
}

#[test]
fn every_shape_of_comment_is_a_comment() {
    // Line, doc, and inner-doc alike: what they have in common is the `//`,
    // and none of the three is code.
    assert_eq!(count("// plain\n/// doc\n//! inner\ncode\n"), 1);
}

#[test]
fn an_indented_comment_is_still_a_comment() {
    assert_eq!(count("code\n    // explaining it\n\t// and again\n"), 1);
}

#[test]
fn code_with_a_trailing_comment_counts() {
    // The line does work, whatever it also says about itself. Only a line that
    // is *nothing but* a comment is free.
    assert_eq!(count("let x = 1; // one\n"), 1);
}

#[test]
fn a_block_comment_is_not_recognised_and_counts_as_code() {
    // Stated as a test so the limitation is a decision on record rather than
    // something a later reader has to infer from an unexpected number.
    assert_eq!(count("/* block */\n"), 1);
}

#[test]
fn windows_line_endings_count_once_each() {
    assert_eq!(count("one\r\ntwo\r\n"), 2);
    // The `\r` must not keep a blank line from reading as blank, nor hide the
    // `//` that starts a comment.
    assert_eq!(count("one\r\n\r\n// two\r\n"), 1);
}
