//! Sign in with Google (#952), against a recorded token endpoint: nothing
//! here reaches Google (CLAUDE.md, *No real provider calls in tests*).

#[path = "../common/mod.rs"]
mod common;

mod access;
mod accounts;
mod config;
mod flow;

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use scorsese_providers::api::google_oauth::Exchange;
use scorsese_providers::credentials::Secret;
use scorsese_providers::sign_in::{Client, SignInError, TokenEndpoint};
use scorsese_server::http::{self, google::Google};
use serde_json::json;
use sqlx::postgres::PgPool;

/// The OAuth client every test signs in with.
const CLIENT_ID: &str = "test-client.apps.googleusercontent.com";

/// Where the test site says it is.
const ORIGIN: &str = "https://scorseseestudio.example";

/// What the token endpoint was sent: code, verifier, redirect URI, secret.
type Sent = (String, String, String, String);

/// Google's token endpoint as a recording: it answers every code with the ID
/// token it holds, and keeps what each exchange sent.
#[derive(Default)]
struct Recorded {
    id_token: Mutex<String>,
    sent: Mutex<Vec<Sent>>,
}

impl Recorded {
    /// A recording answering with a token for `sub` and `email`, verified.
    fn answering(sub: &str, email: &str) -> Arc<Self> {
        let recorded = Arc::new(Self::default());
        recorded.answer(id_token(sub, email, true));
        recorded
    }

    fn answer(&self, id_token: String) {
        *self.id_token.lock().expect("not poisoned") = id_token;
    }

    fn sent(&self) -> Vec<Sent> {
        self.sent.lock().expect("not poisoned").clone()
    }
}

impl TokenEndpoint for Recorded {
    fn exchange(&self, exchange: &Exchange<'_>) -> Result<String, SignInError> {
        self.sent.lock().expect("not poisoned").push((
            exchange.code.to_owned(),
            exchange.code_verifier.to_owned(),
            exchange.redirect_uri.to_owned(),
            exchange.client_secret.to_owned(),
        ));
        Ok(self.id_token.lock().expect("not poisoned").clone())
    }
}

/// An ID token as Google's token endpoint returns one, for this client and
/// an hour from now. The signature is junk: it is never checked.
fn id_token(sub: &str, email: &str, verified: bool) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_secs();
    let part = |value: serde_json::Value| URL_SAFE_NO_PAD.encode(value.to_string());
    let claims = json!({
        "iss": "https://accounts.google.com",
        "aud": CLIENT_ID,
        "sub": sub,
        "email": email,
        "email_verified": verified,
        "name": "Ana Souza",
        "iat": now,
        "exp": now + 3600,
    });
    format!(
        "{}.{}.c2lnbmF0dXJl",
        part(json!({"alg": "RS256", "typ": "JWT"})),
        part(claims)
    )
}

/// A server signing people in through `endpoint` — or, with `None`, one with
/// no Google client configured.
async fn serve(pool: PgPool, endpoint: Option<Arc<Recorded>>) -> SocketAddr {
    let (listener, address) = common::listener().await;
    scorsese_server::db::migrate(&pool)
        .await
        .expect("the migrations apply");
    let members = scorsese_server::db::member_pool(&pool)
        .await
        .expect("the member pool connects");
    let google = endpoint.map(|endpoint| Google {
        client: Client {
            id: CLIENT_ID.to_owned(),
            secret: Secret::new("test-secret"),
        },
        origin: ORIGIN.to_owned(),
        endpoint,
    });
    let state = http::AppState::new(members.clone(), common::files("google")).with_google(google);
    let router = http::router(state);
    let server = tokio::spawn(http::serve(listener, router, std::future::pending()));
    common::release_at_teardown(&pool, &members, [server.abort_handle()]);
    address
}
