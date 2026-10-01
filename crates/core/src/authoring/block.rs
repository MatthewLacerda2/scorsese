//! Changing a caption's `reveal` or `number` block, and taking one away.
//!
//! **A block merges one level down, and it is the only field that can be
//! removed.** Every other field of an [`Edit`](super::Edit) is a value with a
//! default, so "absent" has a meaning already. A block is different: the text
//! either carries one or it does not, and *not* is a real state — a caption
//! that stops counting, or that simply shows. So a change to a block is either
//! [`BlockChange::Merge`], which sets the fields it names over what the block was
//! (over the defaults, when there was none) and keeps the rest, or
//! [`BlockChange::Remove`]. "Type it out letter by letter" is then one field —
//! `unit` — and the rise and stagger somebody tuned stay where they were.

use serde::Deserialize;

use crate::text::{Counter, Locale, Reveal, RevealUnit};

/// What happens to one of a text's optional blocks.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockChange<T> {
    /// The fields named are set; every other field keeps what the block had,
    /// or its default when the text carried no block at all.
    Merge(T),
    /// The block is taken off the text.
    Remove,
}

/// The fields of a [`Reveal`] block to change. An absent one is left alone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevealEdit {
    /// Which pieces the text arrives in.
    pub unit: Option<RevealUnit>,
    /// How far below its place each piece starts, as a fraction of the size.
    pub rise: Option<f64>,
    /// How far one piece gets in before the next one starts, `0` to `1`.
    pub stagger: Option<f64>,
}

impl RevealEdit {
    /// `reveal` with the named fields set over it.
    pub(super) fn over(self, reveal: Reveal) -> Reveal {
        Reveal {
            unit: self.unit.unwrap_or(reveal.unit),
            rise: self.rise.unwrap_or(reveal.rise),
            stagger: self.stagger.unwrap_or(reveal.stagger),
        }
    }
}

/// The fields of a [`Counter`] block to change. An absent one is left alone.
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CounterEdit {
    /// The figure shown when no `number` track animates it.
    pub value: Option<f64>,
    /// How many digits after the decimal mark.
    pub decimals: Option<u8>,
    /// Whose separators the figure is written with.
    pub locale: Option<Locale>,
    /// Whether thousands are grouped.
    pub grouping: Option<bool>,
}

impl CounterEdit {
    /// `counter` with the named fields set over it.
    pub(super) fn over(self, counter: Counter) -> Counter {
        Counter {
            value: self.value.unwrap_or(counter.value),
            decimals: self.decimals.unwrap_or(counter.decimals),
            locale: self.locale.unwrap_or(counter.locale),
            grouping: self.grouping.unwrap_or(counter.grouping),
        }
    }
}

/// `block` after `change`: merged over what it was or its defaults, or gone.
pub(super) fn changed<T: Default, E: Copy>(
    block: Option<T>,
    change: &BlockChange<E>,
    over: fn(E, T) -> T,
) -> Option<T> {
    match change {
        BlockChange::Merge(fields) => Some(over(*fields, block.unwrap_or_default())),
        BlockChange::Remove => None,
    }
}
