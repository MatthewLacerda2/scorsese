//! Naming a file for the browser to save: `Content-Disposition: attachment`.
//!
//! The name is the user's own words, so it can hold anything — quotes,
//! slashes, accents, emoji, a line break pasted in. The header carries it
//! twice, as RFC 6266 says: `filename="…"` with an ASCII stand-in for any
//! browser that reads only that, and `filename*=UTF-8''…` percent-encoded
//! (RFC 8187), which every current browser prefers and which keeps the name
//! exactly as the user wrote it. Neither form can break out of the header: the
//! quoted one holds no quote, backslash or control character, and the encoded
//! one holds nothing but RFC 8187's `attr-char`s and `%XX`.

use std::fmt::Write as _;

use axum::http::HeaderValue;

/// `attachment` with `name`, its extension added unless it already ends in
/// it — an upload is usually named `clip.mp4`, a generation `Narration`.
pub(super) fn attachment(name: &str, extension: &str) -> HeaderValue {
    let name = saved_name(name, extension);
    let value = format!(
        "attachment; filename=\"{}\"; filename*=UTF-8''{}",
        ascii(&name),
        encoded(&name)
    );
    // Every byte above is visible ASCII, so this cannot fail; the fallback is
    // there so that a mistake here costs a name, never a download.
    HeaderValue::from_str(&value).unwrap_or(HeaderValue::from_static("attachment"))
}

/// The name a file is saved under: its own, trimmed, with no path in it, and
/// ending in its real extension.
pub(super) fn saved_name(name: &str, extension: &str) -> String {
    // A slash would make a browser that honours it reach for a folder; none
    // should, but the name is no worse without one.
    let base: String = name
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let base = base.trim();
    let base = if base.is_empty() { "file" } else { base };
    if extension.is_empty() {
        return base.to_owned();
    }
    let suffix = format!(".{extension}");
    let has_it = base.len() > suffix.len()
        && base
            .get(base.len() - suffix.len()..)
            .is_some_and(|end| end.eq_ignore_ascii_case(&suffix));
    if has_it {
        base.to_owned()
    } else {
        format!("{base}{suffix}")
    }
}

/// The name in printable ASCII, each other character as `_`, and no `"` or
/// `\` to end the quoted string early.
fn ascii(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '"' | '\\' => '_',
            c if c.is_ascii_graphic() || c == ' ' => c,
            _ => '_',
        })
        .collect()
}

/// The name as RFC 8187 `value-chars`: `attr-char`s as they are, every other
/// byte of its UTF-8 as `%XX`.
fn encoded(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for byte in name.bytes() {
        let attr_char = byte.is_ascii_alphanumeric()
            || matches!(
                byte,
                b'!' | b'#' | b'$' | b'&' | b'+' | b'-' | b'.' | b'^' | b'_' | b'`' | b'|' | b'~'
            );
        if attr_char {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_keeps_or_gains_its_extension() {
        assert_eq!(saved_name("clip.mp4", "mp4"), "clip.mp4");
        assert_eq!(saved_name("Clip.MP4", "mp4"), "Clip.MP4");
        assert_eq!(saved_name("Narration", "mp3"), "Narration.mp3");
        assert_eq!(saved_name("theme.mid.bak", "mid"), "theme.mid.bak.mid");
        assert_eq!(saved_name(".mp4", "mp4"), ".mp4.mp4");
        assert_eq!(saved_name("  ", "png"), "file.png");
        assert_eq!(saved_name("../etc/passwd", "png"), ".._etc_passwd.png");
    }

    #[test]
    fn the_header_carries_an_ascii_stand_in_and_the_exact_name() {
        let value = attachment("Café \"night\"\n🎬", "mp4");
        assert_eq!(
            value.to_str().unwrap(),
            "attachment; filename=\"Caf_ _night___.mp4\"; \
             filename*=UTF-8''Caf%C3%A9%20%22night%22_%F0%9F%8E%AC.mp4"
        );
    }
}
