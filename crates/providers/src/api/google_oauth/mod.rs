//! Google's OAuth 2.0 / OpenID Connect endpoints: signing a person in with
//! their Google account (#952).
//!
//! Two endpoints. The **authorization endpoint** is where a browser is sent to
//! choose an account and agree; it is a URL built here, never called from
//! here. The **token endpoint** is called from the server, back-channel, to
//! trade the one-time code the browser comes back with for an ID token.
//!
//! What is particular to this vendor, checked against
//! <https://developers.google.com/identity/openid-connect/openid-connect> on
//! 2026-10-13:
//!
//! - **The token endpoint takes a form**, `application/x-www-form-urlencoded`,
//!   with `code`, `client_id`, `client_secret`, `redirect_uri` and
//!   `grant_type`. PKCE's `code_verifier` rides beside them: the page's flow
//!   does not mention it, but its discovery document lists
//!   `code_challenge_methods_supported: ["plain", "S256"]`, and RFC 7636 §4.5
//!   names the parameter.
//! - **`iss` is "always `https://accounts.google.com` or
//!   `accounts.google.com`"** — both are accepted ([`ISSUERS`]).
//! - **`sub` is "unique among all Google Accounts and never reused"**, which
//!   is why it, not the email, is the identity once an account is linked.
//! - `prompt=select_account` makes Google ask which account even when the
//!   browser is signed in to one, so somebody with a work and a personal
//!   account picks rather than gets one.

use serde::Deserialize;

use crate::api::http::{Caller, HttpError, encoded};

/// Where a browser is sent to sign in.
pub const AUTHORIZE: &str = "https://accounts.google.com/o/oauth2/v2/auth";

/// Where the server trades a code for tokens.
pub const TOKEN: &str = "https://oauth2.googleapis.com/token";

/// Every `iss` a Google ID token is documented to carry.
pub const ISSUERS: [&str; 2] = ["https://accounts.google.com", "accounts.google.com"];

/// What is asked for: who they are, and their email.
pub const SCOPE: &str = "openid email";

/// The query string's parameters, in Google's names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authorize<'a> {
    /// The OAuth client's id.
    pub client_id: &'a str,
    /// Where Google sends the browser back; must be registered on the client.
    pub redirect_uri: &'a str,
    /// The value the callback must hand back unchanged.
    pub state: &'a str,
    /// PKCE: the base64url SHA-256 of the verifier (`S256`).
    pub code_challenge: &'a str,
}

impl Authorize<'_> {
    /// The URL to send the browser to.
    pub fn url(&self) -> String {
        let pairs = [
            ("client_id", self.client_id),
            ("redirect_uri", self.redirect_uri),
            ("response_type", "code"),
            ("scope", SCOPE),
            ("state", self.state),
            ("code_challenge", self.code_challenge),
            ("code_challenge_method", "S256"),
            ("prompt", "select_account"),
        ];
        let query: Vec<String> = pairs
            .iter()
            .map(|(name, value)| format!("{name}={}", encoded(value)))
            .collect();
        format!("{AUTHORIZE}?{}", query.join("&"))
    }
}

/// The token request's form, in Google's names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchange<'a> {
    /// The code the browser came back with.
    pub code: &'a str,
    /// The OAuth client's id.
    pub client_id: &'a str,
    /// The OAuth client's secret.
    pub client_secret: &'a str,
    /// The same redirect URI the authorization request named.
    pub redirect_uri: &'a str,
    /// PKCE: the verifier the challenge was made from.
    pub code_verifier: &'a str,
}

/// The token endpoint's reply: only the part read.
///
/// The access token beside it is not kept: nothing here calls a Google API on
/// the person's behalf, so there is nothing to spend it on.
#[derive(Debug, Clone, Deserialize)]
pub struct Tokens {
    /// The ID token: a JWT whose payload says who signed in.
    pub id_token: String,
}

/// Trade `exchange`'s code for tokens.
pub fn exchange(exchange: &Exchange<'_>) -> Result<Tokens, HttpError> {
    let form = [
        ("code", exchange.code),
        ("client_id", exchange.client_id),
        ("client_secret", exchange.client_secret),
        ("redirect_uri", exchange.redirect_uri),
        ("grant_type", "authorization_code"),
        ("code_verifier", exchange.code_verifier),
    ];
    // The client secret travels in the form, so the caller signs nothing.
    Caller::anonymous().post_form(TOKEN, &form)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_authorization_url_carries_every_parameter_escaped() {
        let url = Authorize {
            client_id: "id.apps.googleusercontent.com",
            redirect_uri: "https://example.com/api/auth/google/callback",
            state: "s",
            code_challenge: "c-_",
        }
        .url();
        assert_eq!(
            url,
            "https://accounts.google.com/o/oauth2/v2/auth?client_id=id.apps.googleusercontent.com\
             &redirect_uri=https%3A%2F%2Fexample.com%2Fapi%2Fauth%2Fgoogle%2Fcallback\
             &response_type=code&scope=openid%20email&state=s&code_challenge=c-_\
             &code_challenge_method=S256&prompt=select_account"
        );
    }
}
