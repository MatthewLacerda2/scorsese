//! A text layer that is drawn again for every frame, because a reveal or a
//! count changes what it looks like.
//!
//! **Only those.** A title that fades, moves or scales is the same pixels for
//! the whole clip and is drawn once — the whole-layer properties act on it
//! afterwards. A `reveal` or a `number` track is the exception, because what it
//! changes is which glyphs there are and where: that is setting type, and only
//! the thing that sets the type can do it. So a [`Typing`] keeps everything the
//! setting needs — the face, the style in pixels, the content — and the frame's
//! resolved [`Properties`](scorsese_compositor::Properties) supply the rest.
//!
//! **A counter keeps room for its widest figure.** Every value the count will
//! show is known from the document — its own value and its track's keyframes,
//! since a count never travels past either ([`scorsese_compositor`] clamps it)
//! — so the widest of them is written once here and every other figure padded
//! to its columns.

use scorsese_compositor::path;
use scorsese_compositor::text::{self, Reveal, Style, Sweep, Unpaintable};
use scorsese_compositor::{Area, Frame, Resolution};
use scorsese_core::{Asset, Clip, Counter};

use super::Typeface;

/// Everything needed to set one text clip at any instant.
#[derive(Debug)]
pub(crate) struct Typing {
    font: Typeface,
    /// The content as written, placeholder and all.
    text: String,
    style: Style,
    /// How it reveals: the block's, or every default when it has none — a
    /// `reveal` track on a text with no block reveals word by word.
    reveal: scorsese_core::Reveal,
    /// How its figure is written, and the widest one it will show.
    counter: Option<(Counter, String)>,
    /// Whether any of that changes over the clip. When it does not, the layer
    /// is drawn once and held like any other.
    animates: bool,
}

impl Typing {
    pub(super) fn new(asset: &Asset, clip: &Clip, font: Typeface, style: Style) -> Self {
        let written = asset.text_style();
        let tracked = |property: &'static str| {
            clip.keyframes
                .iter()
                .filter(move |track| track.property.as_str() == property)
        };
        let counter = written.number.map(|counter| {
            let values = std::iter::once(counter.value).chain(
                tracked(path::NUMBER).flat_map(|track| track.keyframes.iter().map(|key| key.value)),
            );
            let widest = values
                .map(|value| counter.format(value))
                .max_by_key(|figure| figure.chars().count())
                .unwrap_or_default();
            (counter, widest)
        });
        let animates = tracked(path::REVEAL).next().is_some()
            || (counter.is_some() && tracked(path::NUMBER).next().is_some());
        Self {
            font,
            text: asset.text.clone().unwrap_or_default(),
            style,
            reveal: written.reveal.unwrap_or_default(),
            counter,
            animates,
        }
    }

    /// Whether this clip's text changes from frame to frame.
    pub(crate) fn animates(&self) -> bool {
        self.animates
    }

    /// The content as it reads at an instant: the figure written in, at
    /// `number` or at the block's own value.
    fn content(&self, number: Option<f64>) -> String {
        let Some((counter, widest)) = &self.counter else {
            return self.text.clone();
        };
        let figure = counter.format(number.unwrap_or(counter.value));
        Counter::fill(&self.text, &text::padded(&figure, widest))
    }

    /// Sets the text as it stands at an instant onto a cleared `frame`.
    pub(crate) fn draw(
        &self,
        frame: &mut Frame,
        sweep: Sweep,
        number: Option<f64>,
    ) -> Vec<Unpaintable> {
        frame.fill_transparent();
        let content = self.content(number);
        if sweep == Sweep::DONE {
            return text::draw(frame, &content, &self.font, &self.style);
        }
        let reveal = Reveal {
            unit: self.reveal.unit,
            // A fraction of the text's own size, which the style already has
            // in pixels.
            rise: (self.reveal.rise * f64::from(self.style.size)) as f32,
            stagger: self.reveal.stagger,
            sweep,
        };
        text::draw_revealing(frame, &content, &self.font, &self.style, &reveal)
    }

    /// Where the block is set: the widest figure's block, since that is the
    /// room the layout keeps.
    pub(super) fn block(&self, resolution: Resolution) -> Area {
        text::block_in(
            &self.content(None),
            &self.font,
            &self.style,
            text::Band::whole(resolution),
            resolution,
        )
    }
}

/// A text asset's content with its figure written in at its own value — what
/// `check` asks the faces about, since digits and separators are characters a
/// face has to have too.
pub(super) fn written(asset: &Asset) -> String {
    let content = asset.text.clone().unwrap_or_default();
    match asset.text_style().number {
        Some(counter) => Counter::fill(&content, &counter.format(counter.value)),
        None => content,
    }
}
