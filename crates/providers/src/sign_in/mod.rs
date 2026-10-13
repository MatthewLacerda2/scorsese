//! Signing a person in with their Google account (#952): who they are, as
//! Google vouches for it.
//!
//! The web app's accounts are Google identities. The server sends a browser
//! to Google's [`authorize_url`], Google sends it back with a one-time code,
//! and the server trades that code for an ID token through a [`TokenEndpoint`]
//! — [`Google`] in the product, a recorded token in every test, so no test
//! reaches Google (CLAUDE.md, *No real provider calls in tests*). What the
//! token says is read by [`identity`], which is the same code for both.
//!
//! **The ID token's signature is not checked**, deliberately. OpenID Connect
//! Core 1.0 §3.1.3.7, item 6: "If the ID Token is received via direct
//! communication between the Client and the Token Endpoint (which it is in
//! this flow), the TLS server validation MAY be used to validate the issuer in
//! place of checking the token signature." Google's own page says the same of
//! its server flow: "since you are communicating directly with Google over an
//! intermediary-free HTTPS channel and using your client secret to
//! authenticate yourself to Google, you can be confident that the token you
//! receive really comes from Google and is valid" (checked 2026-10-13). The
//! token reaches [`identity`] only from [`Google::exchange`]'s own HTTPS call,
//! never from a browser, so checking it against Google's JWKS would buy a key
//! fetch, a cache and an RSA dependency for nothing. What is still checked is
//! what TLS does not say: that it was issued by Google (`iss`), **for this
//! client** (`aud`), that it has not expired (`exp`), and that Google has
//! verified the email (`email_verified`) — an unverified one is not allowed
//! to claim an account that has that email.
//!
//! This module knows nothing of accounts, cookies or sessions: those are the
//! server's. It answers one question — *whom did Google just sign in* — the
//! way [`crate::chat`] answers *what did the model say*.

mod claims;

pub use claims::{Identity, identity};

use crate::api::google_oauth::{self, Authorize, Exchange};
use crate::credentials::Secret;

/// An OAuth client registered with Google: a *Web application* client from
/// the Google Cloud console (`docs/web.md`, *Sign in with Google*).
#[derive(Debug, Clone)]
pub struct Client {
    /// The client's id, `….apps.googleusercontent.com`. Not a secret: it is
    /// in every authorization URL.
    pub id: String,
    /// The client's secret. Never printed.
    pub secret: Secret,
}

/// Why nobody was signed in.
#[derive(Debug, thiserror::Error)]
pub enum SignInError {
    /// Google refused the code, or could not be reached.
    #[error("Google did not trade the sign-in code: {0}")]
    Exchange(#[from] crate::api::http::HttpError),

    /// The ID token is not a JWT whose payload we can read.
    #[error("Google's ID token is unreadable: {0}")]
    Unreadable(String),

    /// The ID token was not issued by Google, or not for this client.
    #[error("the ID token was issued by {iss} for {aud}, not by Google for this client")]
    Foreign {
        /// Its `iss`.
        iss: String,
        /// Its `aud`.
        aud: String,
    },

    /// The ID token has expired.
    #[error("the ID token expired")]
    Expired,

    /// Google has not verified the email, or gave none.
    #[error("Google has not verified this account's email address")]
    Unverified,
}

/// Where a code is traded for an ID token: Google's token endpoint, or a
/// test's recording of one.
pub trait TokenEndpoint: Send + Sync {
    /// The raw ID token `exchange`'s code is worth. Blocking: it is one HTTPS
    /// call, made off the async runtime by whoever holds this.
    fn exchange(&self, exchange: &Exchange<'_>) -> Result<String, SignInError>;
}

/// Google's own token endpoint.
#[derive(Debug, Clone, Copy, Default)]
pub struct Google;

impl TokenEndpoint for Google {
    fn exchange(&self, exchange: &Exchange<'_>) -> Result<String, SignInError> {
        Ok(google_oauth::exchange(exchange)?.id_token)
    }
}

/// The URL that starts `client`'s sign-in, coming back to `redirect_uri` with
/// `state`, and bound to the PKCE verifier whose `S256` challenge is
/// `code_challenge`.
pub fn authorize_url(
    client: &Client,
    redirect_uri: &str,
    state: &str,
    code_challenge: &str,
) -> String {
    Authorize {
        client_id: &client.id,
        redirect_uri,
        state,
        code_challenge,
    }
    .url()
}

/// The form that trades `code` for `client`'s tokens, with the PKCE
/// `verifier` and the same `redirect_uri` the sign-in started with.
pub fn exchange_form<'a>(
    client: &'a Client,
    redirect_uri: &'a str,
    code: &'a str,
    verifier: &'a str,
) -> Exchange<'a> {
    Exchange {
        code,
        client_id: &client.id,
        client_secret: client.secret.expose(),
        redirect_uri,
        code_verifier: verifier,
    }
}
