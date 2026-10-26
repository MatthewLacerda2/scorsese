//! `POST /api/client-errors` (#1428): the browser telling the server it
//! crashed.
//!
//! On 2026-10-16 the editor went black for every Portuguese and Spanish user
//! whenever the assistant worked, for twelve days, and the server never knew:
//! every request it saw answered `200`. Now the web app's error boundaries and
//! its `window` error handlers post what threw here, and the server writes one
//! `client error: …` warning to its log — the log is the whole
//! record; no table keeps them.
//!
//! Anyone may post, signed in or not, since a page can crash before anyone
//! signs in. So the route guards itself: at most [`REPORTS_PER_MINUTE`] from
//! one address ([`Visitor`], the address the access record keeps), a body of
//! at most [`CAP`] bytes, and nothing of the body ever sent back. Each field
//! is written escaped, so a report can never forge a second log line.

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;

use super::AppState;
use super::error::ApiError;
use super::visitor::Visitor;

/// Reports one address may send in a minute; the rest are refused `429`.
pub const REPORTS_PER_MINUTE: u32 = 20;

/// The most a report may weigh, in bytes; a heavier one is refused `413`
/// before it is read.
pub const CAP: usize = 16 * 1024;

/// What the browser says threw. Only `message` is required: an error thrown
/// as a string has no stack, and one outside React no component stack.
#[derive(Debug, Deserialize)]
pub struct Report {
    message: String,
    stack: Option<String>,
    component_stack: Option<String>,
    /// The page's path when it threw.
    path: Option<String>,
    /// The language the page was in.
    language: Option<String>,
    /// Which build of the web app was running.
    build: Option<String>,
}

/// `POST /api/client-errors`: log the report, answer `204`.
pub async fn report(
    State(state): State<AppState>,
    Visitor(address): Visitor,
    report: Result<Json<Report>, axum::extract::rejection::JsonRejection>,
) -> Result<StatusCode, ApiError> {
    if let Err(wait) = state.reports.take(address, 1) {
        let retry_after = wait.as_secs().max(1);
        return Err(ApiError::TooMany {
            message: format!(
                "too many crash reports this minute; try again in {retry_after} seconds"
            ),
            retry_after,
        });
    }
    // The rejection's own words can quote the body back; these never do.
    let Json(report) = report.map_err(|rejection| match rejection.status() {
        StatusCode::PAYLOAD_TOO_LARGE => ApiError::Refused {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            message: format!("a crash report is at most {CAP} bytes"),
            detail: serde_json::json!({}),
        },
        _ => ApiError::BadRequest("a crash report is a JSON object with a `message`".into()),
    })?;
    tracing::warn!("{}", line(&report));
    Ok(StatusCode::NO_CONTENT)
}

/// The log line for `report`: one line, every field quoted and escaped.
fn line(report: &Report) -> String {
    let or_none = |field: &Option<String>| field.as_deref().unwrap_or("").to_owned();
    format!(
        "client error: {:?} at {:?} (language {:?}, build {:?}); stack {:?}; components {:?}",
        report.message,
        or_none(&report.path),
        or_none(&report.language),
        or_none(&report.build),
        or_none(&report.stack),
        or_none(&report.component_stack),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_is_one_line_however_many_lines_it_carries() {
        let report = Report {
            message: "x is not a function\nclient error: forged".into(),
            stack: Some("at Busy\nat Chat".into()),
            component_stack: None,
            path: Some("/projects/7/edit".into()),
            language: Some("pt-BR".into()),
            build: None,
        };
        let line = line(&report);
        assert!(!line.contains('\n'), "{line}");
        assert!(line.starts_with("client error: \"x is not a function\\n"));
        assert!(line.contains("\"/projects/7/edit\"") && line.contains("\"pt-BR\""));
    }
}
