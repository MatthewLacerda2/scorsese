//! The assistant's rates, and the arithmetic that turns a usage block into
//! micro-dollars.

use scorsese_providers::chat::Model;
use scorsese_providers::prices::Checked;
use scorsese_providers::prices::chat::{self, Rate, Usage};

/// The day the table was read, when every row but the dated one applies.
const READ: Checked = Checked::on(2026, 10, 3);

fn rate(model: Model) -> Rate {
    chat::rate(model, READ).expect("every model offered is priced")
}

/// Every figure on the vendors' pages, written out rather than read from the
/// table — a test that derives its expectations from the thing under test
/// proves only that it agrees with itself. In micro-dollars per million.
#[test]
fn the_table_says_what_the_vendors_pages_say() {
    let figures = |r: Rate| {
        (
            r.input,
            r.output,
            r.cache_write_5m,
            r.cache_write_1h,
            r.cache_read,
        )
    };
    let opus = (4_000_000, 20_000_000, 5_000_000, 8_000_000, 200_000);
    assert_eq!(figures(rate(Model::ClaudeOpus55)), opus);
    let sonnet = (2_000_000, 10_000_000, 2_500_000, 4_000_000, 200_000);
    assert_eq!(figures(rate(Model::ClaudeSonnet55)), sonnet);
    let flash = (750_000, 3_750_000, 0, 0, 75_000);
    assert_eq!(figures(rate(Model::GeminiFlash38)), flash);
    let lite = (300_000, 2_500_000, 0, 0, 300_000);
    assert_eq!(figures(rate(Model::GeminiFlashLite35)), lite);
}

/// Gemini 3.8 Flash's introductory price holds through 2026-12-31 and doubles
/// on 2027-01-01 (#718), to $1.50 / $7.50 / $0.15 cached — written out, not
/// read from the table. Nothing else moves that day.
#[test]
fn gemini_flash_doubles_on_new_years_day_2027() {
    let flash = |day| chat::rate(Model::GeminiFlash38, day).expect("priced");
    let figures = |r: Rate| (r.input, r.output, r.cache_read);
    let last_intro_day = flash(Checked::on(2026, 12, 31));
    assert_eq!(figures(last_intro_day), (750_000, 3_750_000, 75_000));
    let first_full_day = flash(Checked::on(2027, 1, 1));
    assert_eq!(figures(first_full_day), (1_500_000, 7_500_000, 150_000));
    assert_eq!(flash(Checked::on(2031, 6, 1)), first_full_day);
    for model in Model::ALL {
        let later = chat::rate(model, Checked::on(2027, 1, 1));
        if model != Model::GeminiFlash38 {
            assert_eq!(later, chat::rate(model, READ), "{model:?}");
        }
    }
}

#[test]
fn every_model_offered_has_a_row_and_its_own_id() {
    for model in Model::ALL {
        assert!(chat::rate(model, READ).is_some(), "{model:?}");
        assert_eq!(Model::from_id(model.id()), Some(model));
    }
    assert_eq!(Model::from_id("claude-opus-5"), None);
    assert_eq!(
        Model::from_id("gemini-3.8-flash-lite"),
        None,
        "there is none"
    );
}

/// A million tokens of one kind only.
fn million(pick: fn(&mut Usage) -> &mut u64) -> u64 {
    let mut usage = Usage::default();
    *pick(&mut usage) = 1_000_000;
    usage.micros(rate(Model::ClaudeOpus55))
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
    assert_eq!(usage.micros(rate(Model::ClaudeOpus55)), 30_000);
}

/// The saving of a cache hit is the user's: the same prompt read from
/// Gemini's cache costs less than read fresh. 40 000 tokens of prompt and 500
/// out on 3.8 Flash — uncached, 40 000 × $0.75/M + 500 × $3.75/M =
/// 30 000 + 1 875 µ$; with 36 000 of them cached, 4 000 × $0.75/M +
/// 36 000 × $0.075/M + 1 875 = 3 000 + 2 700 + 1 875 µ$.
#[test]
fn a_cached_gemini_call_costs_less_than_the_same_call_uncached() {
    let flash = rate(Model::GeminiFlash38);
    let fresh = Usage {
        input: 40_000,
        output: 500,
        ..Usage::default()
    };
    let cached = Usage {
        input: 4_000,
        cache_read: 36_000,
        ..fresh
    };
    assert_eq!(fresh.micros(flash), 31_875);
    assert_eq!(cached.micros(flash), 7_575);
}

/// One Opus cache-read token is a fifth of a micro-dollar; it is charged as a
/// whole one, and a call is rounded up to the micro-dollar above.
#[test]
fn a_fraction_rounds_up_once() {
    let opus = rate(Model::ClaudeOpus55);
    let read = Usage {
        cache_read: 1,
        ..Usage::default()
    };
    assert_eq!(read.micros(opus), 1);
    let both = Usage { output: 1, ..read };
    assert_eq!(both.micros(opus), 21, "20.2 µ$ is 21");
    assert_eq!(Usage::default().micros(opus), 0);
}
