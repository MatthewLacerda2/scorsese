//! The kept exchange rate (`fx::Latest`) from outside the crate (#1654).
//!
//! Written only against what `Latest` and `Rates` had before #1654, so CI's
//! `regression test` job can run it on the base and watch it fail there.

use std::sync::{Arc, Mutex};

use scorsese_providers::fx::{FxError, Latest, Rate, Rates, Source};

/// A source answering each of its figures in turn, the last for ever, with
/// no PTAX to check against.
struct Answers(Mutex<Vec<&'static str>>);

impl Rates for Answers {
    fn usd_brl(&self) -> Result<Rate, FxError> {
        let mut answers = self.0.lock().expect("answers");
        let text = if answers.len() > 1 {
            answers.remove(0)
        } else {
            answers[0]
        };
        Ok(Rate::parse(text, Source::Awesome).expect("a plausible rate"))
    }
}

#[test]
fn a_rate_far_from_the_last_kept_is_not_kept() {
    let latest = Latest::new(Arc::new(Answers(Mutex::new(vec!["5.30", "2.00"]))));
    latest.refresh().expect("5.30 is kept");
    assert!(latest.refresh().is_err(), "2.00 is no rate after 5.30");
    assert_eq!(latest.current().expect("a rate kept").brl_per_usd, "5.30");
}
