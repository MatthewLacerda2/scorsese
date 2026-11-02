//! What a call may cost, and the longest reply what is left pays for
//! (#1653): what `holds` decides each call on.
//!
//! A call's input is priced at the most it can cost — every token written to
//! the cache it is sent for ([`Input`]), as a call that finds nothing cached
//! writes it — and its reply at [`OUTPUT_HELD`] tokens, or as many as what is
//! left pays for ([`most_out`]), up to [`MAX_TOKENS`].

use scorsese_providers::chat::{MAX_TOKENS, Model, Request};
use scorsese_providers::prices::Checked;
use scorsese_providers::prices::chat::{self, Rate, Usage};

use crate::credits::{Product, price};

/// Output tokens a call is held for: a few steps of thinking and a tool
/// call, about what an editing call writes, though one may write up to its
/// `holds::Held::most`. Input is held at the most it can cost, written to the
/// caches ([`Input`]), where a cached call pays a twentieth of the input
/// rate for most of it. It is also the shortest reply a call is made for.
pub(super) const OUTPUT_HELD: u64 = 4_096;

/// Bytes of a request's text taken for one token: four, the estimate
/// `tool_calls` already records replies by (migration `0020`).
const BYTES_PER_TOKEN: usize = 4;

/// Tokens a picture in the request is held for: Anthropic's largest
/// unscaled image, about 1.15 megapixels at 750 pixels a token — the figure
/// `tools::size` takes for a picture it cannot measure. Its base64 is never
/// counted as text, which would make one picture weigh tens of thousands.
const PICTURE_TOKENS: usize = 1_600;

/// What starts a picture's (or a document's) base64 in the wire's JSON: its
/// `data` key. A string's own quotes are escaped, so text that says this is
/// never mistaken for one.
const BASE64_KEY: &str = "\"data\":\"";

/// The least a call on `model` may cost: a reply of [`OUTPUT_HELD`] tokens,
/// with no input at all — what a turn must have left to be begun.
pub(super) fn least(model: Model) -> i64 {
    held(model, Input::default())
}

/// What `request` may cost the user: its input at the most it can cost,
/// written to the caches ([`Input`]) — text at [`BYTES_PER_TOKEN`], each
/// picture at [`PICTURE_TOKENS`] — and [`OUTPUT_HELD`] tokens out, times the
/// assistant's multiplier. Nothing for a model with no price, which is never
/// charged either. With the input tokens it counted.
pub(super) fn estimate(request: &Request) -> (Input, i64) {
    let input = input_of(request);
    (input, held(request.model, input))
}

/// The tokens a request sends, by the cache each may be written to: the
/// system prompt and tools, cached for an hour, and the conversation after
/// them, for five minutes (`chat::claude::request`). A call that finds
/// neither cached writes both, at each cache's write rate — the most its
/// input can cost, which is what it is priced at.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Input {
    /// The system prompt and the tools.
    prefix: u64,
    /// The messages.
    rest: u64,
}

/// The tokens `request` sends, as [`estimate`] counts them.
fn input_of(request: &Request) -> Input {
    let tools: usize = request
        .tools
        .iter()
        .map(|tool| tool.name.len() + tool.description.len() + tool.schema.to_string().len())
        .sum();
    let (text, pictures) = request
        .messages
        .iter()
        .map(|message| weigh(message.get()))
        .fold((0, 0), |(text, pictures), (t, p)| (text + t, pictures + p));
    let tokens = |n: usize| u64::try_from(n).unwrap_or(u64::MAX);
    Input {
        prefix: tokens((request.system.len() + tools) / BYTES_PER_TOKEN),
        rest: tokens(text / BYTES_PER_TOKEN + pictures * PICTURE_TOKENS),
    }
}

/// One message's text bytes and the pictures in it: everything but each
/// base64 string after [`BASE64_KEY`], which is one picture.
fn weigh(json: &str) -> (usize, usize) {
    let (mut text, mut pictures, mut rest) = (0, 0, json);
    while let Some(at) = rest.find(BASE64_KEY) {
        let start = at + BASE64_KEY.len();
        text += start;
        pictures += 1;
        // base64 has no quote or backslash, so the next quote ends it.
        rest = rest[start..]
            .find('"')
            .map_or("", |end| &rest[start + end..]);
    }
    (text + rest.len(), pictures)
}

/// What `input` and [`OUTPUT_HELD`] out cost on `model` today, to the user.
fn held(model: Model, input: Input) -> i64 {
    chat::rate(model, Checked::today()).map_or(0, |rate| priced(rate, input, OUTPUT_HELD))
}

/// What `input`, written to the caches ([`Input`]), and `output` out cost at
/// `rate`, to the user.
fn priced(rate: Rate, input: Input, output: u64) -> i64 {
    let usage = Usage {
        input: 0,
        output,
        cache_write_5m: input.rest,
        cache_write_1h: input.prefix,
        cache_read: 0,
    };
    price(Product::Assistant, usage.micros(rate))
}

/// The longest reply on `model`, after `input` tokens in, that `left` pays
/// for today: [`MAX_TOKENS`] when it pays for that, and never less than
/// [`OUTPUT_HELD`], which a call is not made without. Unbounded for a model
/// with no price, which is never charged.
pub(super) fn most_out(model: Model, input: Input, left: i64) -> u32 {
    let Some(rate) = chat::rate(model, Checked::today()) else {
        return MAX_TOKENS;
    };
    let pays = |output: u64| priced(rate, input, output) <= left;
    let (mut paid, mut unpaid) = (OUTPUT_HELD, u64::from(MAX_TOKENS));
    if pays(unpaid) {
        return MAX_TOKENS;
    }
    while unpaid - paid > 1 {
        let middle = paid + (unpaid - paid) / 2;
        if pays(middle) {
            paid = middle;
        } else {
            unpaid = middle;
        }
    }
    u32::try_from(paid).unwrap_or(MAX_TOKENS)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `prefix` tokens of system prompt and tools, `rest` of messages.
    fn input(prefix: u64, rest: u64) -> Input {
        Input { prefix, rest }
    }

    #[test]
    fn a_hold_errs_high_and_grows_with_what_is_sent() {
        let model = Model::DEFAULT;
        let rate = chat::rate(model, Checked::today()).expect("the default model is priced");
        let nothing = held(model, Input::default());
        assert_eq!(
            nothing,
            price(Product::Assistant, OUTPUT_HELD * rate.output / 1_000_000)
        );
        assert!(held(model, input(0, 100_000)) > nothing);
        let request = |text: &str| Request {
            model,
            system: text.to_owned(),
            tools: Vec::new(),
            messages: Vec::new(),
            max_tokens: MAX_TOKENS,
        };
        assert_eq!(estimate(&request("")).1, nothing);
        assert_eq!(
            estimate(&request(&"x".repeat(400_000))),
            (input(100_000, 0), held(model, input(100_000, 0)))
        );
    }

    #[test]
    fn input_is_priced_at_the_most_it_can_cost_written_to_the_caches() {
        let model = Model::DEFAULT;
        let rate = chat::rate(model, Checked::today()).expect("the default model is priced");
        let (prefix, rest) = (100_000, 50_000);
        let written = priced(rate, input(prefix, rest), 0);
        let cost = prefix * rate.cache_write_1h + rest * rate.cache_write_5m;
        assert_eq!(written, price(Product::Assistant, cost.div_ceil(1_000_000)));
        let uncached = prefix.saturating_add(rest) * rate.input;
        assert!(
            u128::from(cost) > u128::from(uncached),
            "a write costs more than plain input"
        );
    }

    #[test]
    fn a_picture_is_held_as_a_picture_and_never_as_its_base64() {
        let message = |json: String| {
            serde_json::value::RawValue::from_string(json).expect("the test's JSON holds")
        };
        let picture = "A".repeat(400_000);
        let with_picture = Request {
            model: Model::DEFAULT,
            system: String::new(),
            tools: Vec::new(),
            messages: vec![message(format!(
                "{{\"role\":\"user\",\"content\":[{{\"type\":\"image\",\"source\":\
                 {{\"type\":\"base64\",\"media_type\":\"image/jpeg\",\"data\":\"{picture}\"}}}}]}}"
            ))],
            max_tokens: MAX_TOKENS,
        };
        let around = with_picture.messages[0].get().len() - picture.len();
        assert_eq!(
            estimate(&with_picture).1,
            held(
                Model::DEFAULT,
                input(
                    0,
                    u64::try_from(around / BYTES_PER_TOKEN + PICTURE_TOKENS).unwrap()
                )
            )
        );
        // Words that merely say the key, escaped inside a string, are words.
        let words = "x".repeat(400);
        let quoting = Request {
            messages: vec![message(format!(
                "{{\"role\":\"user\",\"content\":\"\\\"data\\\":\\\"{words}\"}}"
            ))],
            ..with_picture
        };
        let length = quoting.messages[0].get().len();
        assert_eq!(
            estimate(&quoting).1,
            held(
                Model::DEFAULT,
                input(0, u64::try_from(length / BYTES_PER_TOKEN).unwrap())
            )
        );
    }

    #[test]
    fn a_reply_is_as_long_as_what_is_left_pays_for() {
        let model = Model::DEFAULT;
        let rate = chat::rate(model, Checked::today()).expect("the default model is priced");
        let input = input(15_000, 5_000);
        let whole = priced(rate, input, u64::from(MAX_TOKENS));
        assert_eq!(most_out(model, input, whole), MAX_TOKENS);
        assert_eq!(most_out(model, input, i64::MAX), MAX_TOKENS);
        let short = most_out(model, input, whole / 2);
        assert!(
            short > u32::try_from(OUTPUT_HELD).unwrap() && short < MAX_TOKENS,
            "{short}"
        );
        assert!(priced(rate, input, u64::from(short)) <= whole / 2);
        assert!(
            priced(rate, input, u64::from(short) + 1) > whole / 2,
            "the longest it pays for"
        );
        // Never shorter than the reply a call is decided on.
        assert_eq!(u64::from(most_out(model, input, 0)), OUTPUT_HELD);
        assert_eq!(least(model), held(model, Input::default()));
    }
}
