//! One sign-in in flight: its `state`, its PKCE verifier and where to land,
//! carried between the two endpoints in a short-lived cookie.
//!
//! **A cookie, not a table.** Both are what RFC 6749 §10.12 and RFC 7636 ask
//! the client to keep between sending the browser to Google and the browser
//! coming back. A row would need a sweep for every sign-in somebody abandoned,
//! and buys nothing a cookie does not: the cookie is unsigned because nothing
//! in it needs to be believed. `state` is proven by matching — the callback
//! accepts only the value this browser's cookie holds, and no other site can
//! set a cookie for this origin, which is what stops a login CSRF (somebody
//! else's code planted in your browser). The verifier is Google's to check
//! against the challenge it was shown, and somebody who altered it would only
//! fail their own sign-in. So, as for sessions, the server keeps **no signing
//! key**.
//!
//! **`SameSite=Lax`, unlike the session's `Strict`.** The callback is a
//! top-level navigation from `accounts.google.com`, a cross-site one, and a
//! Strict cookie is not sent on it. Lax is, on a top-level `GET`, which is
//! exactly the callback. It lives ten minutes and only under
//! `/api/auth/google`, and the callback clears it whatever happens.

use axum::http::HeaderMap;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

use crate::accounts::{AccountError, secret};
use crate::http::auth::cookie;

/// The cookie's name.
pub const FLOW_COOKIE: &str = "scorsese_google";

/// How long a sign-in may take, in seconds: long enough to pick an account
/// and type a Google password, short enough that a stale one is not a key.
pub const LIFETIME: u32 = 10 * 60;

/// Where a sign-in lands when it was not sent from somewhere in particular.
pub const HOME: &str = "/projects";

const ATTRIBUTES: &str = "Path=/api/auth/google; HttpOnly; Secure; SameSite=Lax";

/// A sign-in in flight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flow {
    /// What Google must hand back unchanged.
    pub state: String,
    /// PKCE's secret half; Google is shown only its [`challenge`](Self::challenge).
    pub verifier: String,
    /// The path to land on once signed in.
    pub next: String,
}

impl Flow {
    /// A new sign-in, landing on `next` when that is a path on this site.
    pub fn mint(next: Option<&str>) -> Result<Self, AccountError> {
        Ok(Self {
            state: secret::mint("")?.plain,
            // 64 hex characters: inside RFC 7636's 43 to 128, all unreserved.
            verifier: secret::mint("")?.plain,
            next: landing(next),
        })
    }

    /// The `S256` code challenge: base64url of the verifier's SHA-256.
    pub fn challenge(&self) -> String {
        URL_SAFE_NO_PAD.encode(Sha256::digest(self.verifier.as_bytes()))
    }

    /// The `Set-Cookie` value that carries this sign-in to the callback.
    pub fn set_cookie(&self) -> String {
        let next = URL_SAFE_NO_PAD.encode(self.next.as_bytes());
        format!(
            "{FLOW_COOKIE}={}.{}.{next}; {ATTRIBUTES}; Max-Age={LIFETIME}",
            self.state, self.verifier
        )
    }

    /// The sign-in this request's cookie carries, if it carries one whole.
    pub fn from_headers(headers: &HeaderMap) -> Option<Self> {
        let value = cookie(headers, FLOW_COOKIE)?;
        let mut parts = value.split('.');
        let (Some(state), Some(verifier), Some(next), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return None;
        };
        let next = String::from_utf8(URL_SAFE_NO_PAD.decode(next).ok()?).ok()?;
        (!state.is_empty() && !verifier.is_empty()).then(|| Self {
            state: state.to_owned(),
            verifier: verifier.to_owned(),
            next: landing(Some(&next)),
        })
    }
}

/// The `Set-Cookie` value that ends a sign-in, done or not.
pub fn clear_cookie() -> String {
    format!("{FLOW_COOKIE}=; {ATTRIBUTES}; Max-Age=0")
}

/// `next` when it is a path on this site, [`HOME`] otherwise — never another
/// origin (`//evil.example` is a URL to another host), so a crafted sign-in
/// link cannot bounce somebody elsewhere. The web app's `safeNext` is the
/// same rule.
pub fn landing(next: Option<&str>) -> String {
    match next {
        Some(next)
            if next.starts_with('/') && !next.starts_with("//") && !next.starts_with("/\\") =>
        {
            next.to_owned()
        }
        _ => HOME.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;
    use axum::http::header::COOKIE;

    use super::*;

    #[test]
    fn a_flow_survives_its_cookie() {
        let flow = Flow::mint(Some("/projects/7/edit?tab=a")).unwrap();
        let set = flow.set_cookie();
        let value = set.split(';').next().unwrap();
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_str(&format!("other=1; {value}")).unwrap(),
        );
        assert_eq!(Flow::from_headers(&headers), Some(flow));
        assert!(set.contains("SameSite=Lax") && set.contains("HttpOnly") && set.contains("Secure"));
    }

    #[test]
    fn the_challenge_is_rfc_7636s_s256() {
        // RFC 7636, appendix B.
        let flow = Flow {
            state: "s".into(),
            verifier: "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".into(),
            next: HOME.into(),
        };
        assert_eq!(
            flow.challenge(),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn only_a_path_on_this_site_is_landed_on() {
        assert_eq!(landing(Some("/library")), "/library");
        for elsewhere in [
            None,
            Some(""),
            Some("https://evil.example"),
            Some("//evil.example"),
            Some("/\\evil"),
        ] {
            assert_eq!(landing(elsewhere), HOME, "{elsewhere:?}");
        }
    }

    #[test]
    fn a_partial_cookie_is_no_flow() {
        let mut headers = HeaderMap::new();
        headers.insert(
            COOKIE,
            HeaderValue::from_static("scorsese_google=onlystate"),
        );
        assert_eq!(Flow::from_headers(&headers), None);
    }
}
