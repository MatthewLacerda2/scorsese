//! Quote first, spend second: the one rule every paid tool here follows.
//!
//! A tool that spends money answers its first call with what it would spend
//! and a token, and spends only when called again with `confirm` set to that
//! token. The rules — bound to exactly what was quoted, single use, fifteen
//! minutes — are [`scorsese_providers::quote`]'s; this is only the wording and
//! the one argument, shared so that `generate` and `voice_design` cannot come
//! to say it differently.
//!
//! **There is no one-step path, locally or anywhere.** The stdio server is run
//! with somebody's own key and spends their own money, which is as real as a
//! credit balance. A shortcut here would also be a second contract for one
//! tool: a client that learned the one-step habit against this server would
//! spend without asking the day it was pointed at the hosted one.

use std::path::Path;

use scorsese_core::Timestamp;
use scorsese_providers::prices::dollars;
use scorsese_providers::quote::{LIFETIME_SECONDS, ProjectQuotes, Quote, issue, redeem};
use serde_json::Value;

use super::Reply;

/// The name of the argument that carries a token back.
pub(crate) const CONFIRM: &str = "confirm";

/// The `confirm` property, described the same way on every paid tool.
pub(crate) fn property() -> Value {
    serde_json::json!({
        "type": "string",
        "description": "The token from this tool's own quote. Leave it out to be quoted — \
                        nothing is sent and no key is needed. Pass it back, after whoever \
                        is paying has agreed to the quoted price, to spend exactly what was \
                        quoted. Good once, for fifteen minutes; if a brief changes in \
                        between, the call is refused and a new quote is needed."
    })
}

/// Permission to spend, or the reply that asks for it.
///
/// `Ok(None)` is *go ahead*: either nothing here would be paid for, or the
/// token handed in was issued for exactly this. `Ok(Some(reply))` is the quote,
/// with a fresh token, and nothing spent. `Err` is a token refused — nothing
/// spent there either, and the sentence says what to do.
pub(crate) fn gate(
    dir: &Path,
    arguments: &Value,
    quote: &Quote,
    tool: &str,
) -> Result<Option<Reply>, String> {
    if quote.is_free() {
        return Ok(None);
    }
    let store = ProjectQuotes::new(dir);
    let now = Timestamp::unix_now().ok_or("the system clock is before 1970")?;
    if let Some(token) = arguments.get(CONFIRM).and_then(Value::as_str) {
        redeem(&store, token.trim(), quote, now).map_err(|refused| format!("{refused}"))?;
        return Ok(None);
    }
    let issued =
        issue(&store, quote, now).map_err(|error| format!("keeping the quote: {error}"))?;
    let mut lines = said(quote);
    lines.push(format!(
        "Nothing has been sent. To spend this, call {tool} again with the same arguments and \
         confirm: \"{}\" — once whoever is paying has agreed to {}. Good once, for {} \
         minutes, for exactly these briefs; if one changes, quote again.",
        issued.token,
        dollars(issued.cents),
        LIFETIME_SECONDS / 60
    ));
    Ok(Some(lines.join("\n").into()))
}

/// The quote as lines: one per item, then the total.
pub(crate) fn said(quote: &Quote) -> Vec<String> {
    let mut lines: Vec<String> = quote
        .items
        .iter()
        .map(|item| format!("{}: {}", item.subject, item.says))
        .collect();
    lines.push(format!(
        "About {} in all — our arithmetic over published rates, never a bill.",
        dollars(quote.cents())
    ));
    lines
}
