//! What a quote's box lists: every brief its token would pay for, with the
//! words each would send (#709), so the user never agrees to money without
//! seeing what it buys.
//!
//! The items are the token's own ([`Pending::items`]), written when the paid
//! tool issued it from the quote the token is bound to — the same briefs a
//! yes spends, priced as it takes them. Nothing here is read back from the
//! reply's words: those only supply the rest of the box's lines (the total,
//! a note), less each line that prices an item already listed.

use super::store::QuoteItem;
use crate::tools::Pending;

/// `lines` — the reply of the call that issued `pending` — as the box shows
/// them: an item for every brief the token covers, and every line not about
/// one of them as it was.
pub(super) fn describe(pending: &Pending, lines: &[String]) -> (Vec<QuoteItem>, Vec<String>) {
    let items = pending.items.clone();
    let rest = lines
        .iter()
        .filter(|line| !prices_an_item(&pending.items, line))
        .cloned()
        .collect();
    (items, rest)
}

/// Whether `line` is the paid tool's line about one of `items`.
fn prices_an_item(items: &[QuoteItem], line: &str) -> bool {
    line.split_once(": ")
        .is_some_and(|(subject, _)| items.iter().any(|item| item.subject == subject))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tools::BriefKind;

    fn item(subject: &str, micros: i64, brief: BriefKind, description: &str) -> QuoteItem {
        QuoteItem {
            subject: subject.to_owned(),
            says: format!("would be generated for {micros}"),
            micros,
            brief,
            description: description.to_owned(),
        }
    }

    fn pending(items: Vec<QuoteItem>) -> Pending {
        Pending {
            token: "quote-1".to_owned(),
            micros: items.iter().map(|item| item.micros).sum(),
            expires_at: 900,
            batch_micros: None,
            items,
        }
    }

    /// The reply names one of the two briefs the token covers; the box still
    /// lists both, in the token's order, and they add up to its price.
    #[test]
    fn a_reply_naming_fewer_items_than_its_token_covers_lists_them_all() {
        let hero = item("hero", 1_200_000, BriefKind::Prompt, "A red cape at dusk.");
        let vo = item(
            "vo",
            300_000,
            BriefKind::Line,
            "Every city has a night editor.",
        );
        let pending = pending(vec![hero.clone(), vo.clone()]);
        let lines = vec![
            "vo: would be generated for 300000".to_owned(),
            "About $1.00 at the providers' published rates — $1.50 from your credits.".to_owned(),
        ];
        let (items, rest) = describe(&pending, &lines);
        assert_eq!(items, vec![hero, vo]);
        let shown: i64 = items.iter().map(|item| item.micros).sum();
        assert_eq!(shown, pending.micros);
        assert_eq!(rest, lines[1..], "the item's line is not said twice");
    }

    /// A line about something the token does not cover — held back, not paid
    /// for — stays a line, and nothing is listed for it.
    #[test]
    fn a_line_about_what_is_not_paid_for_stays_a_line() {
        let hero = item("hero", 1_200_000, BriefKind::Prompt, "A red cape at dusk.");
        let pending = pending(vec![hero.clone()]);
        let lines = vec![
            "hero: would be generated for 1200000".to_owned(),
            "narrator: no voice chosen yet, so it is skipped".to_owned(),
        ];
        let (items, rest) = describe(&pending, &lines);
        assert_eq!(items, vec![hero]);
        assert_eq!(rest, lines[1..]);
    }
}
