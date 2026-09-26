//! The assistant's rates, and the arithmetic that turns a `usage` block into
//! micro-dollars.

use scorsese_providers::prices::claude::{self, MODEL, Usage};

/// Every figure on Anthropic's page for Claude Opus 5.5, written out rather
/// than read from the table — a test that derives its expectations from the
/// thing under test proves only that it agrees with itself.
#[test]
fn the_table_says_what_the_vendors_page_says() {
    let rate = claude::rate(MODEL).expect("the assistant's model is priced");
    assert_eq!(rate.input, 400, "$4 per million");
    assert_eq!(rate.output, 2000, "$20 per million");
    assert_eq!(rate.cache_write_5m, 500, "$5 per million");
    assert_eq!(rate.cache_write_1h, 800, "$8 per million");
    assert_eq!(rate.cache_read, 20, "$0.20 per million");
}

#[test]
fn a_model_nobody_priced_has_no_rate() {
    assert_eq!(claude::rate("claude-opus-5"), None);
}

/// A million tokens of one kind only.
fn million(pick: fn(&mut Usage) -> &mut u64) -> u64 {
    let mut usage = Usage::default();
    *pick(&mut usage) = 1_000_000;
    usage.micros(claude::rate(MODEL).expect("the assistant's model is priced"))
}

#[test]
fn a_million_of_each_kind_costs_its_rate() {
    assert_eq!(million(|u| &mut u.input), 4_000_000);
    assert_eq!(million(|u| &mut u.output), 20_000_000);
    assert_eq!(million(|u| &mut u.cache_write_5m), 5_000_000);
    assert_eq!(million(|u| &mut u.cache_write_1h), 8_000_000);
    assert_eq!(million(|u| &mut u.cache_read), 200_000);
}

/// A typical assistant turn: a large cached prefix, a little new input, a
/// few hundred tokens out. Worked by hand: 50 000 × $0.20/M = 10 000 µ$,
/// 2 000 × $4/M = 8 000 µ$, 600 × $20/M = 12 000 µ$.
#[test]
fn a_turn_adds_every_kind() {
    let usage = Usage {
        input: 2_000,
        output: 600,
        cache_read: 50_000,
        ..Usage::default()
    };
    assert_eq!(usage.micros(claude::rate(MODEL).unwrap()), 30_000);
}

/// One cache-read token is a fifth of a micro-dollar; it is charged as a
/// whole one, and a call is rounded up to the micro-dollar above.
#[test]
fn a_fraction_rounds_up_once() {
    let rate = claude::rate(MODEL).unwrap();
    let read = Usage {
        cache_read: 1,
        ..Usage::default()
    };
    assert_eq!(read.micros(rate), 1);
    let both = Usage { output: 1, ..read };
    assert_eq!(both.micros(rate), 21, "20.2 µ$ is 21");
    assert_eq!(Usage::default().micros(rate), 0);
}
