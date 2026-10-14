//! Where the dollar's price in reais is read (#548): AwesomeAPI's commercial
//! quote, and the Central Bank's PTAX as its fallback. Neither takes a key.
//!
//! Checked on 2026-10-14 against <https://docs.awesomeapi.com.br/api-de-moedas>
//! and the BCB's OData service at `olinda.bcb.gov.br`, by calling both:
//!
//! - **AwesomeAPI** answers `GET /json/last/USD-BRL` with
//!   `{"USDBRL": {"bid": "5.7276", "ask": "5.7282", …}}`, **every figure a
//!   string**. `ask` is what a dollar costs to buy. Without a key the answer
//!   is cached for a minute and the free quota is small: a call from a cloud
//!   machine that day answered `429 QuotaExceeded`, which is why there is a
//!   fallback at all.
//! - **The BCB's PTAX** answers `CotacaoDolarPeriodo` with
//!   `{"value": [{"cotacaoVenda": 5.0096, "dataHoraCotacao": "…"}, …]}`, one
//!   row per business day and **the figures as JSON numbers**, which are read
//!   here as their text so no float ever holds one. A day with no bulletin yet
//!   (a weekend, a holiday, the morning before 13:00) has no row, so the last
//!   week is asked for and the newest row is the rate. Dates are `MM-DD-YYYY`.

use serde::Deserialize;
use serde_json::value::RawValue;

use crate::api::http::{Caller, HttpError, encoded};

/// AwesomeAPI's latest USD→BRL quote.
pub const AWESOME: &str = "https://economia.awesomeapi.com.br/json/last/USD-BRL";

/// The BCB's PTAX service.
const PTAX: &str = "https://olinda.bcb.gov.br/olinda/servico/PTAX/versao/v1/odata";

/// AwesomeAPI's reply.
#[derive(Debug, Deserialize)]
struct Awesome {
    #[serde(rename = "USDBRL")]
    usd_brl: AwesomeQuote,
}

/// One pair's quote.
#[derive(Debug, Deserialize)]
struct AwesomeQuote {
    ask: String,
}

/// AwesomeAPI's selling price of a dollar, as the text it was sent as.
pub fn awesome_ask() -> Result<String, HttpError> {
    let reply: Awesome = Caller::anonymous().get(AWESOME)?;
    Ok(reply.usd_brl.ask)
}

/// The PTAX service's reply.
#[derive(Debug, Deserialize)]
struct Ptax<'a> {
    #[serde(borrow)]
    value: Vec<PtaxRow<'a>>,
}

/// One day's bulletin.
#[derive(Debug, Deserialize)]
struct PtaxRow<'a> {
    #[serde(rename = "cotacaoVenda", borrow)]
    selling: &'a RawValue,
    /// `YYYY-MM-DD HH:MM:SS.ffffff`, which sorts as text.
    #[serde(rename = "dataHoraCotacao")]
    when: String,
}

/// The URL asking for every PTAX bulletin from `first` to `last`, each
/// `MM-DD-YYYY`.
pub fn ptax_url(first: &str, last: &str) -> String {
    format!(
        "{PTAX}/CotacaoDolarPeriodo(dataInicial=@dataInicial,dataFinalCotacao=@dataFinalCotacao)\
         ?@dataInicial={}&@dataFinalCotacao={}&$format=json&$select=cotacaoVenda,dataHoraCotacao",
        encoded(&format!("'{first}'")),
        encoded(&format!("'{last}'")),
    )
}

/// The newest PTAX selling rate between `first` and `last`, as the text it was
/// sent as, or `None` when the period has no bulletin.
pub fn ptax_selling(first: &str, last: &str) -> Result<Option<String>, HttpError> {
    let url = ptax_url(first, last);
    let bytes = Caller::anonymous().download(&url, 1024 * 1024)?;
    newest_selling(&bytes).map_err(|message| HttpError::Unreadable { url, message })
}

/// The newest row's `cotacaoVenda` in a PTAX reply.
fn newest_selling(bytes: &[u8]) -> Result<Option<String>, String> {
    let reply: Ptax<'_> = serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let newest = reply.value.iter().max_by(|a, b| a.when.cmp(&b.when));
    Ok(newest.map(|row| row.selling.get().to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn awesomes_reply_reads_its_ask_as_text() {
        let reply: Awesome = serde_json::from_str(
            r#"{"USDBRL": {"code": "USD", "codein": "BRL", "bid": "5.7276",
                "ask": "5.7282", "timestamp": "1618315045"}}"#,
        )
        .unwrap();
        assert_eq!(reply.usd_brl.ask, "5.7282");
    }

    #[test]
    fn the_newest_ptax_row_is_read_exactly_as_sent() {
        // The service's reply on 2026-10-14, trailing zero and all.
        let reply = br#"{"@odata.context": "x", "value": [
            {"cotacaoVenda": 5.00960, "dataHoraCotacao": "2026-10-13 13:04:26.866133"},
            {"cotacaoVenda": 4.98920, "dataHoraCotacao": "2026-10-09 13:07:30.746597"}]}"#;
        assert_eq!(newest_selling(reply).unwrap().as_deref(), Some("5.00960"));
        assert_eq!(newest_selling(br#"{"value": []}"#).unwrap(), None);
        assert!(newest_selling(b"<html>").is_err());
    }

    #[test]
    fn the_ptax_url_quotes_its_dates() {
        let url = ptax_url("10-07-2026", "10-14-2026");
        assert!(url.contains("@dataInicial=%2710-07-2026%27"), "{url}");
        assert!(url.contains("@dataFinalCotacao=%2710-14-2026%27"), "{url}");
    }
}
