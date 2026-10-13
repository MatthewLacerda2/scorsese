//! Reading an ID token: the JWT's payload, and the four checks TLS does not
//! make for us (the module doc above has why the signature is not one).

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::Deserialize;

use super::SignInError;
use crate::api::google_oauth::ISSUERS;

/// Whom Google signed in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    /// Google's id for the account: never reused, and what an account is
    /// known by once linked.
    pub sub: String,
    /// Their email, which Google has verified.
    pub email: String,
}

/// The claims read, in the names OpenID Connect gives them.
#[derive(Debug, Deserialize)]
struct Claims {
    iss: String,
    aud: String,
    sub: String,
    exp: i64,
    email: Option<String>,
    /// A boolean, though older Google tokens sent the string `"true"`.
    #[serde(default)]
    email_verified: Option<serde_json::Value>,
}

/// Whom `id_token` names, if it was issued by Google for `client_id`, is
/// unexpired at `now` (seconds since the epoch), and carries a verified email.
pub fn identity(id_token: &str, client_id: &str, now: i64) -> Result<Identity, SignInError> {
    let claims = payload(id_token)?;
    if !ISSUERS.contains(&claims.iss.as_str()) || claims.aud != client_id {
        return Err(SignInError::Foreign {
            iss: claims.iss,
            aud: claims.aud,
        });
    }
    if claims.exp <= now {
        return Err(SignInError::Expired);
    }
    let verified = matches!(&claims.email_verified, Some(serde_json::Value::Bool(true)))
        || matches!(&claims.email_verified, Some(serde_json::Value::String(s)) if s == "true");
    match claims.email {
        Some(email) if verified && !email.is_empty() && !claims.sub.is_empty() => Ok(Identity {
            sub: claims.sub,
            email,
        }),
        _ => Err(SignInError::Unverified),
    }
}

/// The middle of `header.payload.signature`, decoded.
fn payload(id_token: &str) -> Result<Claims, SignInError> {
    let mut parts = id_token.split('.');
    let (Some(_), Some(payload), Some(_), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(SignInError::Unreadable(
            "not three dot-separated parts".to_owned(),
        ));
    };
    let bytes = URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .map_err(|error| SignInError::Unreadable(error.to_string()))?;
    serde_json::from_slice(&bytes).map_err(|error| SignInError::Unreadable(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIENT: &str = "client.apps.googleusercontent.com";

    fn token(claims: serde_json::Value) -> String {
        let part = |value: &serde_json::Value| URL_SAFE_NO_PAD.encode(value.to_string());
        let header = serde_json::json!({"alg": "RS256", "typ": "JWT"});
        format!("{}.{}.c2lnbmF0dXJl", part(&header), part(&claims))
    }

    fn claims() -> serde_json::Value {
        serde_json::json!({
            "iss": "https://accounts.google.com",
            "aud": CLIENT,
            "sub": "110169484474386276334",
            "email": "ana@example.com",
            "email_verified": true,
            "exp": 2_000,
        })
    }

    #[test]
    fn a_google_token_for_this_client_names_its_account() {
        let identity = identity(&token(claims()), CLIENT, 1_000).unwrap();
        assert_eq!(identity.sub, "110169484474386276334");
        assert_eq!(identity.email, "ana@example.com");
        let mut bare = claims();
        bare["iss"] = "accounts.google.com".into();
        bare["email_verified"] = "true".into();
        assert!(super::identity(&token(bare), CLIENT, 1_000).is_ok());
    }

    #[test]
    fn another_issuer_or_audience_is_foreign() {
        let mut other = claims();
        other["aud"] = "someone-else".into();
        let refused = identity(&token(other), CLIENT, 1_000);
        assert!(matches!(refused, Err(SignInError::Foreign { .. })));
        let mut other = claims();
        other["iss"] = "https://evil.example".into();
        let refused = identity(&token(other), CLIENT, 1_000);
        assert!(matches!(refused, Err(SignInError::Foreign { .. })));
    }

    #[test]
    fn an_expired_token_is_refused_at_its_expiry() {
        let refused = identity(&token(claims()), CLIENT, 2_000);
        assert!(matches!(refused, Err(SignInError::Expired)));
        assert!(identity(&token(claims()), CLIENT, 1_999).is_ok());
    }

    #[test]
    fn an_unverified_or_missing_email_is_refused() {
        for (key, value) in [
            ("email_verified", serde_json::json!(false)),
            ("email_verified", serde_json::json!("false")),
            ("email_verified", serde_json::Value::Null),
            ("email", serde_json::Value::Null),
            ("email", serde_json::json!("")),
        ] {
            let mut unverified = claims();
            unverified[key] = value;
            let refused = identity(&token(unverified), CLIENT, 1_000);
            assert!(matches!(refused, Err(SignInError::Unverified)), "{key}");
        }
    }

    #[test]
    fn a_token_that_is_not_a_jwt_is_unreadable() {
        for garbage in ["", "a.b", "a.!!!.c", "a.b.c.d"] {
            let refused = identity(garbage, CLIENT, 1_000);
            assert!(
                matches!(refused, Err(SignInError::Unreadable(_))),
                "{garbage}"
            );
        }
    }
}
