//! *The assistant*: each chat model's five token rates and the day they were read.

use scorsese_providers::chat::Model;
use scorsese_providers::prices::{Checked, chat};

use super::{code, micros, section, table};

const HEADER: &str =
    "| model | input | output | cache write, 5 min | cache write, 1 h | cache read | checked |";

#[test]
fn every_token_rate_is_the_codes() {
    let rows = table(section("The assistant"), HEADER);
    let mut named: Vec<&str> = rows.iter().map(|row| code(&row[0])).collect();
    let mut offered: Vec<&str> = Model::ALL.map(Model::id).to_vec();
    named.sort_unstable();
    offered.sort_unstable();
    assert_eq!(named, offered, "the assistant table's models");
    for row in &rows {
        let model = Model::ALL
            .into_iter()
            .find(|model| model.id() == code(&row[0]))
            .expect("named above");
        let rate = chat::rate(model, Checked::today()).expect("every model is priced");
        let coded = [
            ("input", rate.input),
            ("output", rate.output),
            ("cache write, 5 min", rate.cache_write_5m),
            ("cache write, 1 h", rate.cache_write_1h),
            ("cache read", rate.cache_read),
        ];
        for ((column, micros_per_million), cell) in coded.iter().zip(&row[1..]) {
            assert_eq!(
                micros(cell),
                *micros_per_million,
                "docs/prices.md, The assistant, `{}`, {column}",
                model.id()
            );
        }
        assert_eq!(
            row[6],
            rate.checked.to_string(),
            "docs/prices.md, The assistant, `{}`, checked",
            model.id()
        );
    }
}
