//! A value made safe for a URL, and a query string made of them.

/// Everything RFC 3986 lets stand unescaped in a query value.
const UNRESERVED: &[u8] = b"-._~";

/// `value`, safe to put in a URL.
///
/// Here, beside the transport, because every vendor module builds a query
/// string and none of them should own the escaping the others use. Written
/// rather than taken as a dependency, for the reason the MCP
/// server gives for its own base64: a dozen lines against a fixed
/// specification, and every dependency is one `cargo deny` has to keep
/// clearing. Deliberately strict — everything outside the unreserved set is
/// escaped, including characters a query would tolerate, because being
/// over-cautious in a URL costs nothing and being wrong costs a failed call.
pub(crate) fn encoded(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        if byte.is_ascii_alphanumeric() || UNRESERVED.contains(byte) {
            out.push(char::from(*byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `pairs` as the query string to hang off a URL — `?name=value&…`, each
/// value [`encoded`] and in the order given — leaving out every pair whose
/// value is `None`, and empty when that leaves none.
pub(crate) fn query(pairs: &[(&str, Option<&str>)]) -> String {
    let present: Vec<String> = pairs
        .iter()
        .filter_map(|(name, value)| Some(format!("{name}={}", encoded((*value)?))))
        .collect();
    match present.is_empty() {
        true => String::new(),
        false => format!("?{}", present.join("&")),
    }
}

/// The host `url` names, lower case, read the way the transport reads it —
/// `None` when it names none.
pub(crate) fn host(url: &str) -> Option<String> {
    let parsed: ureq::http::Uri = url.parse().ok()?;
    parsed.host().map(str::to_ascii_lowercase)
}

/// The host of `url`, lower case, when `url` is plain https to it: no user
/// name or password before the host and no port but 443. Read the way the
/// transport reads it, so the host answered is the one that would be called.
pub(crate) fn https_host(url: &str) -> Option<String> {
    let parsed: ureq::http::Uri = url.parse().ok()?;
    let authority = parsed.authority()?;
    let plain = parsed.scheme_str() == Some("https")
        && !authority.as_str().contains('@')
        && parsed.port_u16().is_none_or(|port| port == 443);
    plain.then(|| authority.host().to_ascii_lowercase())
}

/// Free text as a query value: trimmed, and `None` when that leaves nothing.
pub(crate) fn filled(value: &Option<String>) -> Option<&str> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::{encoded, filled, query};

    #[test]
    fn only_the_unreserved_set_stands_unescaped() {
        assert_eq!(encoded("a-Z_9.~"), "a-Z_9.~");
        assert_eq!(encoded("são paulo&x=1"), "s%C3%A3o%20paulo%26x%3D1");
    }

    #[test]
    fn a_query_keeps_its_order_and_leaves_out_what_is_absent() {
        assert_eq!(
            query(&[("q", Some("red car")), ("kind", None), ("page", Some("2"))]),
            "?q=red%20car&page=2"
        );
    }

    #[test]
    fn blank_text_is_no_value() {
        assert_eq!(filled(&Some(" british ".into())), Some("british"));
        assert_eq!(filled(&Some("  ".into())), None);
        assert_eq!(filled(&None), None);
    }

    #[test]
    fn a_query_of_nothing_is_empty() {
        assert_eq!(query(&[]), "");
        assert_eq!(query(&[("q", None)]), "");
    }
}
