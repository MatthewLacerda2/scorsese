//! The two endpoints a browser passes through, as a browser would.

use std::net::SocketAddr;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use sqlx::postgres::PgPool;

use super::common::{Response, request};
use super::{ORIGIN, Recorded, id_token, serve};

/// Every `Set-Cookie` line's `name=value`.
fn cookies(response: &Response) -> Vec<String> {
    let set = response.headers.iter().filter_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.eq_ignore_ascii_case("set-cookie")
            .then(|| value.trim().to_owned())
    });
    set.collect()
}

/// The value of `name` in a URL's query.
fn query<'a>(url: &'a str, name: &str) -> &'a str {
    let query = url.split_once('?').expect("a URL with a query").1;
    let pair = query
        .split('&')
        .find(|pair| pair.starts_with(&format!("{name}=")));
    let pair = pair.unwrap_or_else(|| panic!("no {name} in {url}"));
    pair.split_once('=').expect("name=value").1
}

/// Start a sign-in landing on `next`: Google's URL, and the flow cookie.
async fn start(address: SocketAddr, next: &str) -> (String, String) {
    let path = format!("/api/auth/google?next={next}");
    let started = request(address, "GET", &path, &[], None).await;
    assert_eq!(started.status, 303, "{}", started.body);
    let flow = cookies(&started).pop().expect("a flow cookie");
    (
        started.header("location").expect("a redirect").to_owned(),
        flow,
    )
}

/// Come back from Google with `code` and `state`, carrying `flow`.
async fn back(address: SocketAddr, state: &str, flow: Option<&str>) -> Response {
    let path = format!("/api/auth/google/callback?code=4%2F0Ab&state={state}");
    let cookie =
        flow.map(|flow| format!("Cookie: {}", flow.split(';').next().expect("name=value")));
    let headers: Vec<&str> = cookie.iter().map(String::as_str).collect();
    request(address, "GET", &path, &headers, None).await
}

#[sqlx::test]
async fn a_sign_in_ends_signed_in_where_it_was_going(pool: PgPool) {
    let google = Recorded::answering("1101", "Ana@Example.com");
    let address = serve(pool, Some(google.clone())).await;
    let (url, flow) = start(address, "%2Flibrary").await;
    assert!(
        url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"),
        "{url}"
    );
    let redirect = format!("{ORIGIN}/api/auth/google/callback")
        .replace(':', "%3A")
        .replace('/', "%2F");
    assert_eq!(query(&url, "redirect_uri"), redirect);
    assert!(
        flow.contains("SameSite=Lax") && flow.contains("Path=/api/auth/google"),
        "{flow}"
    );

    let signed = back(address, query(&url, "state"), Some(&flow)).await;
    assert_eq!(signed.status, 303, "{}", signed.body);
    assert_eq!(signed.header("location"), Some("/library"));
    let set = cookies(&signed);
    let session = set
        .iter()
        .find(|c| c.starts_with("scorsese_session="))
        .unwrap();
    for attribute in [
        "HttpOnly",
        "Secure",
        "SameSite=Strict",
        "Path=/api",
        "Max-Age=2592000",
    ] {
        assert!(session.contains(attribute), "{session}");
    }
    assert!(
        set.iter()
            .any(|c| c.starts_with("scorsese_google=;") && c.contains("Max-Age=0"))
    );
    let cookie = format!("Cookie: {}", session.split(';').next().unwrap());
    let me = request(address, "GET", "/api/me", &[cookie.as_str()], None).await;
    assert_eq!(me.json()["email"], "ana@example.com");

    // The server proved the code with the verifier behind the challenge it showed.
    let (code, verifier, redirect_uri, secret) = google.sent().pop().unwrap();
    assert_eq!((code.as_str(), secret.as_str()), ("4/0Ab", "test-secret"));
    assert_eq!(redirect_uri, format!("{ORIGIN}/api/auth/google/callback"));
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    assert_eq!(query(&url, "code_challenge"), challenge);
}

#[sqlx::test]
async fn a_callback_without_its_own_flow_signs_nobody_in(pool: PgPool) {
    let google = Recorded::answering("1101", "ana@example.com");
    let address = serve(pool, Some(google.clone())).await;
    let (url, flow) = start(address, "").await;
    let state = query(&url, "state");
    for (state, flow) in [("forged", Some(flow.as_str())), (state, None)] {
        let refused = back(address, state, flow).await;
        assert_eq!(refused.header("location"), Some("/login?error=expired"));
        assert!(
            !cookies(&refused)
                .iter()
                .any(|c| c.starts_with("scorsese_session="))
        );
    }
    assert!(google.sent().is_empty(), "no code was traded");
}

#[sqlx::test]
async fn every_other_way_out_lands_on_the_login_saying_why(pool: PgPool) {
    let google = Recorded::answering("1101", "ana@example.com");
    let address = serve(pool, Some(google.clone())).await;
    let (url, flow) = start(address, "").await;
    let cookie = format!("Cookie: {}", flow.split(';').next().unwrap());
    let path = "/api/auth/google/callback?error=access_denied";
    let cancelled = request(address, "GET", path, &[cookie.as_str()], None).await;
    assert_eq!(cancelled.header("location"), Some("/login?error=cancelled"));

    google.answer(id_token("1101", "ana@example.com", false));
    let unverified = back(address, query(&url, "state"), Some(&flow)).await;
    assert_eq!(
        unverified.header("location"),
        Some("/login?error=unverified")
    );
    google.answer("not a jwt".to_owned());
    let failed = back(address, query(&url, "state"), Some(&flow)).await;
    assert_eq!(failed.header("location"), Some("/login?error=failed"));
}

#[sqlx::test]
async fn a_server_without_a_google_client_says_so(pool: PgPool) {
    let unconfigured = serve(pool, None).await;
    for path in [
        "/api/auth/google",
        "/api/auth/google/callback?code=a&state=b",
    ] {
        let refused = request(unconfigured, "GET", path, &[], None).await;
        assert_eq!(
            refused.header("location"),
            Some("/login?error=unconfigured")
        );
    }
}
