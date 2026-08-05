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
//!
//! [`Refusal`] is the other half: where [`read`] says what the vendor *said*,
//! that says what it **means**, and it is what the text-to-speech path acts on.

use serde_json::Value;

/// The key is valid and lacks a permission the call needs.
pub const MISSING_PERMISSIONS: &str = "missing_permissions";

/// The account's plan does not include what was asked for.
pub const PAID_PLAN_REQUIRED: &str = "paid_plan_required";

/// The account has no credits left.
pub const QUOTA_EXCEEDED: &str = "quota_exceeded";

/// Too many of our own requests are in flight at once.
pub const TOO_MANY_CONCURRENT: &str = "too_many_concurrent_requests";

/// The vendor itself is loaded.
pub const SYSTEM_BUSY: &str = "system_busy";

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
    /// The vendor's finer cause: `quota_exceeded`, `system_busy`.
    ///
    /// Finer than `status` and coarser than `message`, and the field to match on
    /// when two refusals share a status code but call for different advice. An
    /// exhausted quota has been seen behind more than one status; its code says
    /// so unambiguously either way.
    pub code: Option<String>,
}

impl Detail {
    /// Whether the vendor's word for this is `status`.
    pub fn is(&self, status: &str) -> bool {
        self.status.as_deref() == Some(status)
    }

    /// Whether the vendor's finer cause is `code`.
    pub fn caused_by(&self, code: &str) -> bool {
        self.code.as_deref() == Some(code)
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
        code: string(detail.get("code")),
    }
}

/// A JSON value as an owned string, when it is one.
fn string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(ToOwned::to_owned)
}

/// Why a request was refused, sorted into what to do about it.
///
/// [`Detail`] above reads what the vendor *said*; this decides what it
/// **means**. The split is worth keeping: a status code is not advice, and four
/// of the cases here arrive as a `401` or a `402` with next steps ranging from
/// *fix your key* through *add a scope to your key* to *this voice needs a paid
/// plan*. Three different actions behind two numbers.
///
/// Every variant carries the vendor's own sentence. That is not padding: on a
/// permissions failure the vendor names the exact scope the key is missing, and
/// no taxonomy written here could reconstruct that.
///
/// **All of it was confirmed against the live API**, which is the only reason
/// the `401` split exists at all. Read from documentation, a `401` is one
/// thing.
///
/// [`voices`](crate::voices) reaches its own conclusions from [`read`] rather
/// than from this, because it predates it and answers a narrower question —
/// *may this voice be used*. The two agree; that they are not yet one call is
/// worth tidying, not worth a rewrite inside a feature branch.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// The key is wrong, or there is not one.
    #[error("ElevenLabs rejected the key: {message} — check ELEVENLABS_API_KEY (see .env.example)")]
    BadKey {
        /// The vendor's sentence.
        message: String,
    },

    /// The key is **right** and was not granted a scope this call needs.
    ///
    /// Its own variant rather than a shade of [`Refusal::BadKey`], and the
    /// distinction is the whole reason this sorting exists. Telling somebody to
    /// check their key when the key is perfectly correct sends them to stare at
    /// the one thing that is not the problem; the fix is a checkbox in the
    /// vendor's dashboard, on a key that already works.
    #[error(
        "the ElevenLabs key is valid but lacks a permission this needs: {message} — \
         add it to the key in the ElevenLabs dashboard; the key itself is fine"
    )]
    MissingPermission {
        /// The vendor's sentence, which names the permission.
        message: String,
    },

    /// The account's plan does not cover this — a voice, or an endpoint.
    ///
    /// Free accounts are refused **per voice** as well as per endpoint: a
    /// library voice answers `402` on a plan that cannot use library voices,
    /// even though the same key speaks perfectly well with another voice. So
    /// this is not "upgrade to use scorsese", it is "that voice needs a paid
    /// plan", and the message says which because the vendor's does.
    #[error("ElevenLabs refused this on the account's current plan: {message}")]
    PlanRequired {
        /// The vendor's sentence.
        message: String,
    },

    /// The account has no credits left.
    ///
    /// Named rather than left as a generic failure because it is the one case
    /// here that is nobody's bug: it is their key, so it is their billing to
    /// act on, and a message that reads like a network problem sends them
    /// looking in the wrong place entirely.
    #[error("the ElevenLabs account is out of credits: {message}")]
    OutOfCredits {
        /// The vendor's sentence.
        message: String,
    },

    /// The voice is not there.
    ///
    /// Expected rather than exceptional. Every one of the vendor's default
    /// voices expires on 2026-12-31 and the set is being replaced before then,
    /// so this is a state the project has to be able to sit in and recover
    /// from — a voice to re-choose, not a run to abandon.
    #[error(
        "ElevenLabs has no voice `{voice_id}` — it may have been withdrawn, \
         or the id may be wrong; choose another voice for this line"
    )]
    VoiceGone {
        /// The id that resolved to nothing.
        voice_id: String,
    },

    /// The request was malformed.
    ///
    /// Should be rare: the length limit, the model and the language coupling
    /// are all checked in the document before anything is sent. One arriving
    /// means a check is missing, so the message is worth reading in full.
    #[error("ElevenLabs would not accept the request: {message}")]
    Invalid {
        /// The vendor's sentence.
        message: String,
    },

    /// Too much at once, or the vendor is having a moment.
    ///
    /// Two causes, one variant, and the difference carried in `wait` because
    /// that is the only part a caller acts on: too many of our own requests
    /// clears in a moment, and a busy service does not.
    #[error("ElevenLabs is not taking this right now ({message}) — {}", wait.advice())]
    Busy {
        /// The vendor's sentence.
        message: String,
        /// How long a wait this is.
        wait: Wait,
    },

    /// Something else, carried whole rather than flattened into a guess.
    #[error("ElevenLabs answered {status}: {message}")]
    Other {
        /// The status code.
        status: u16,
        /// Whatever the vendor said.
        message: String,
    },
}

/// Whether a busy vendor is busy for a moment or for a while.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wait {
    /// Our own requests are stacked up. Clears as they finish.
    Moment,
    /// The service is loaded. Nothing we do makes it clear sooner.
    While,
}

impl Wait {
    /// What to do about it, as a message says it.
    pub const fn advice(self) -> &'static str {
        match self {
            Self::Moment => "retry shortly; this clears as requests in flight finish",
            Self::While => "retry later; the service itself is loaded",
        }
    }
}

impl Refusal {
    /// Sorts a refusal out of the status and body the vendor sent.
    ///
    /// The `code` is consulted before the status, not after, because the two
    /// disagree about which fact matters: an exhausted quota has been seen
    /// behind more than one status, and its code says so unambiguously either
    /// way.
    pub fn of(status: u16, voice_id: &str, body: &str) -> Self {
        let detail = read(body);
        let message = detail.says(body).to_owned();

        if detail.caused_by(QUOTA_EXCEEDED) {
            return Self::OutOfCredits { message };
        }
        if detail.caused_by(TOO_MANY_CONCURRENT) {
            return Self::Busy {
                message,
                wait: Wait::Moment,
            };
        }
        if detail.caused_by(SYSTEM_BUSY) {
            return Self::Busy {
                message,
                wait: Wait::While,
            };
        }

        match status {
            401 if detail.is(MISSING_PERMISSIONS) => Self::MissingPermission { message },
            401 => Self::BadKey { message },
            402 => Self::PlanRequired { message },
            404 => Self::VoiceGone {
                voice_id: voice_id.to_owned(),
            },
            422 => Self::Invalid { message },
            429 => Self::Busy {
                message,
                wait: Wait::While,
            },
            status => Self::Other { status, message },
        }
    }

    /// Whether trying this again, unchanged, could succeed.
    ///
    /// What separates a run worth resuming from one worth editing first. A
    /// missing scope and a spent quota both say *no* here even though both are
    /// fixable, because neither is fixed by scorsese trying harder.
    pub const fn is_worth_retrying(&self) -> bool {
        matches!(self, Self::Busy { .. })
    }
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
