//! The sentence ElevenLabs puts inside a refusal, and the word beside it.
//!
//! A status code is not enough to act on here, and that is not a theoretical
//! complaint. A `401` from this vendor covers two situations that call for
//! opposite advice:
//!
//! - `"status": "missing_permissions"` — **the key is perfectly good** and
//!   simply was not granted a scope. The body names the exact permission
//!   (`voices_read`), and the fix is in the vendor's dashboard.
//! - a genuinely absent or wrong key — the fix is in `.env` or the settings
//!   file.
//!
//! Reporting the first as the second sends somebody to stare at a credential
//! that is already correct, which is the most expensive kind of wrong error
//! message: it is confidently specific and it points the wrong way.
//!
//! So the body is read, not just the number. Only two fields are taken —
//! `detail.status` and `detail.message` — and both are optional, because
//! `detail` is not always an object at all: a validation failure puts an
//! *array* of field errors there. Everything unrecognised reads as "no detail",
//! and a caller that gets none falls back to the status code, which is exactly
//! what it would have had anyway.

use serde_json::Value;

/// The key is valid and lacks a permission the call needs.
pub const MISSING_PERMISSIONS: &str = "missing_permissions";

/// The account's plan does not include what was asked for.
pub const PAID_PLAN_REQUIRED: &str = "paid_plan_required";

/// What the vendor said about a refusal, where it said anything readable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Detail {
    /// The vendor's own word for the situation: `missing_permissions`,
    /// `paid_plan_required`, `invalid_api_key`.
    pub status: Option<String>,
    /// The sentence a person should read. **Carried whole and never
    /// summarised** — it is where the vendor names the missing permission, and
    /// paraphrasing it would drop the one detail that makes it actionable.
    pub message: Option<String>,
}

impl Detail {
    /// Whether the vendor's word for this is `status`.
    pub fn is(&self, status: &str) -> bool {
        self.status.as_deref() == Some(status)
    }

    /// The best sentence available about this refusal.
    ///
    /// The vendor's own where there is one, and `fallback` — the whole response
    /// body — where there is not. Never empty: a refusal a caller cannot quote
    /// is one nobody can debug.
    pub fn says<'a>(&'a self, fallback: &'a str) -> &'a str {
        self.message
            .as_deref()
            .filter(|message| !message.trim().is_empty())
            .unwrap_or(fallback)
    }
}

/// What a refusal body says, as far as it is readable.
///
/// Never fails. A body that is not JSON, or is JSON of another shape, yields an
/// empty [`Detail`] rather than an error — the caller already has the status
/// code and the body itself, so failing to parse an extra hint is not a failure
/// worth propagating.
pub fn read(body: &str) -> Detail {
    let Ok(Value::Object(root)) = serde_json::from_str::<Value>(body) else {
        return Detail::default();
    };
    let Some(Value::Object(detail)) = root.get("detail") else {
        return Detail::default();
    };
    Detail {
        status: string(detail.get("status")),
        message: string(detail.get("message")),
    }
}

/// A JSON value as an owned string, when it is one.
fn string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The body the live API answered with when the key existed but had not
    /// been granted `voices_read` — the case this module was written for.
    const MISSING: &str = concat!(
        r#"{"detail":{"type":"authentication_error","code":"unauthorized","#,
        r#""message":"The API key you used is missing the permission voices_read "#,
        r#"to execute this operation.","status":"missing_permissions"}}"#
    );

    #[test]
    fn a_missing_permission_is_recognised_and_quoted_whole() {
        let detail = read(MISSING);
        assert!(detail.is(MISSING_PERMISSIONS));
        assert!(detail.says("").contains("voices_read"));
    }

    /// `detail` is an array on a validation failure, and that must read as no
    /// detail rather than as a parse error.
    #[test]
    fn a_detail_that_is_not_an_object_is_simply_no_detail() {
        let detail = read(r#"{"detail":[{"loc":["body","text"],"msg":"field required"}]}"#);
        assert_eq!(detail, Detail::default());
    }

    #[test]
    fn a_body_that_is_not_json_at_all_is_no_detail() {
        assert_eq!(read("<html>502 Bad Gateway</html>"), Detail::default());
        assert_eq!(read("").says("the body was empty"), "the body was empty");
    }
}
