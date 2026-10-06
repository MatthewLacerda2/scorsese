use super::compact;

#[test]
fn whitespace_between_tokens_goes() {
    let pretty = "{\n  \"a\": [\n    1,\n    2\n  ],\r\n\t\"b\": null\n}\n";
    assert_eq!(compact(pretty), r#"{"a":[1,2],"b":null}"#);
}

#[test]
fn whitespace_inside_strings_stays() {
    let pretty = "{ \"name\": \"Two  words\\tand a\\nline\" }";
    assert_eq!(compact(pretty), r#"{"name":"Two  words\tand a\nline"}"#);
}

/// An escaped quote does not end a string, and an escaped backslash before a
/// real closing quote does not keep it open.
#[test]
fn escapes_do_not_confuse_where_a_string_ends() {
    let pretty = r#"{ "a": "say \"hi there\"", "b": "C:\\ ", "c": 1 }"#;
    assert_eq!(
        compact(pretty),
        r#"{"a":"say \"hi there\"","b":"C:\\ ","c":1}"#
    );
}

/// Key order is the document's, not sorted — the reason this is not a parse
/// and re-serialise.
#[test]
fn key_order_is_kept() {
    assert_eq!(compact(r#"{ "z": 1, "a": 2 }"#), r#"{"z":1,"a":2}"#);
}

#[test]
fn text_that_is_not_json_comes_back_untouched() {
    let broken = "{\n  \"a\": 1,\n  oops\n}";
    assert_eq!(compact(broken), broken);
}

#[test]
fn the_same_value_comes_back() {
    let pretty = "{\n  \"a\": [1, {\"b\": \"x y\"}],\n  \"é\": 2.5e3\n}";
    let before: serde_json::Value = serde_json::from_str(pretty).expect("valid");
    let after: serde_json::Value = serde_json::from_str(&compact(pretty)).expect("valid");
    assert_eq!(before, after);
}
