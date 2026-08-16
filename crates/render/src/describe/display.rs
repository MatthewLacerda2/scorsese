//! How a description reads.
//!
//! The reader is an agent deciding whether the cut is right, so every line is
//! one stretch of timeline or one thing inside it, the two times are always
//! side by side, and nothing is left to be inferred from a field name. A
//! description that has to be parsed to be understood is a description nobody
//! reads.

use std::fmt;

use scorsese_core::{AssetKind, Crop, Easing, Fit, Geometry, Shape, Speed};

use super::{Animated, Description, Playing, Shown, Stretch, Travel};

/// How much of a prompt or a title a line carries before it is cut short. Long
/// enough for a sentence, short enough that a stack of cards stays a table.
const PROMPT_WIDTH: usize = 72;

impl fmt::Display for Description {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:.2}s, {} frames at {} fps, timeline frames {}–{}, {} stretch{}",
            self.seconds(),
            self.out_frames,
            self.out_fps,
            self.from.frame.get(),
            self.to.frame.get(),
            self.stretches.len(),
            if self.stretches.len() == 1 { "" } else { "es" }
        )?;
        if self.is_silent() {
            write!(f, ", no soundtrack")?;
        }
        // Silence is only worth naming in a film that has sound somewhere;
        // saying it under every stretch of a slideshow would double the output
        // to repeat what the header already said once.
        let name_silence = !self.is_silent();
        for stretch in &self.stretches {
            f.write_str("\n\n")?;
            stretch.write(f, name_silence)?;
        }
        Ok(())
    }
}

impl fmt::Display for Stretch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write(f, true)
    }
}

impl Stretch {
    /// One stretch, with `silence` deciding whether an empty mix is named or
    /// simply absent.
    fn write(&self, f: &mut fmt::Formatter<'_>, silence: bool) -> fmt::Result {
        write!(
            f,
            "  {:<14} f{}–{}",
            format!("{:.2}–{:.2}s", self.from.seconds, self.to.seconds),
            self.from.frame.get(),
            self.to.frame.get()
        )?;
        if self.picture.is_empty() {
            write!(f, "\n    picture  gap — black")?;
        }
        for playing in &self.picture {
            line(f, "picture", playing, &onscreen(playing))?;
        }
        if self.sound.is_empty() && silence {
            write!(f, "\n    sound    silence")?;
        }
        for playing in &self.sound {
            line(f, "sound", playing, &audible(playing))?;
        }
        Ok(())
    }
}

/// One clip's line, with whatever it animates underneath it.
fn line(f: &mut fmt::Formatter<'_>, medium: &str, playing: &Playing, tail: &str) -> fmt::Result {
    write!(
        f,
        "\n    {medium:<8} {:<16} {tail}",
        format!("{}/{}", playing.track, playing.clip)
    )?;
    for animated in &playing.animated {
        write!(f, "\n        {animated}")?;
    }
    Ok(())
}

/// What a clip contributes to the picture: what it shows, and how it meets the
/// raster.
fn onscreen(playing: &Playing) -> String {
    match &playing.shows {
        // A card is drawn at the raster's own size, so `fit` never touches it
        // and printing one would be inventing a fact about the render.
        Shown::Card { absent, prompt } => {
            format!("{} (card, {absent}) \"{}\"", playing.asset, clipped(prompt))
        }
        Shown::Text(text) => format!(
            "{} (text, {}) \"{}\"",
            playing.asset,
            fit(playing.fit),
            clipped(text)
        ),
        // Like a card, a colour is the raster itself rather than something
        // fitted into it, so printing a `fit` would be inventing a fact.
        Shown::Color(color) => format!("{} (color {color})", playing.asset),
        // A shape is drawn at the raster's own size for the same reason, so
        // there is no `fit` to print here either. What replaces it is the
        // shape itself: a diagram is many small layers, and "shape" alone
        // would not tell two of them apart.
        Shown::Shape(shape) => format!("{} ({})", playing.asset, drawn(shape)),
        Shown::Media => format!(
            "{} ({}, {}{}{})",
            playing.asset,
            kind(playing.kind),
            fit(playing.fit),
            cropped(playing.crop),
            retimed(playing.speed)
        ),
    }
}

/// What a clip contributes to the mix — including when the answer is nothing,
/// which is the finding worth having.
fn audible(playing: &Playing) -> String {
    let what = match &playing.shows {
        // The prompt is already on this clip's picture line; a card in the mix
        // is only here to say that what it stands in for cannot be heard.
        Shown::Card { absent, .. } => format!("{} (card, {absent})", playing.asset),
        _ => format!(
            "{} ({}{})",
            playing.asset,
            kind(playing.kind),
            retimed(playing.speed)
        ),
    };
    if playing.is_audible() {
        what
    } else {
        format!("{what} — silent")
    }
}

impl fmt::Display for Animated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.travel {
            Some(travel) => write!(
                f,
                "{} {:.2} → {:.2} {travel}",
                self.property, self.at_start, self.at_end
            ),
            None => write!(f, "{} {:.2} held", self.property, self.at_start),
        }
    }
}

impl fmt::Display for Travel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "over {:.2}–{:.2}s (f{}–{}, {})",
            self.from.seconds,
            self.to.seconds,
            self.from.frame.get(),
            self.to.frame.get(),
            self.easing.map_or("mixed", easing)
        )
    }
}

/// One line's worth of a string: control characters flattened, and a tail too
/// long for a table cut with an ellipsis.
fn clipped(text: &str) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let flat = flat.trim().to_owned();
    if flat.chars().count() <= PROMPT_WIDTH {
        return flat;
    }
    let kept: String = flat.chars().take(PROMPT_WIDTH - 1).collect();
    format!("{}…", kept.trim_end())
}

/// The crop, when there is one, in the words the document writes it.
///
/// Nothing at all when the clip shows its whole source, which is nearly every
/// clip: a line reading "crop: none" on every shot would bury the one shot
/// where the answer is interesting.
fn cropped(crop: Option<Crop>) -> String {
    crop.map_or_else(String::new, |crop| {
        format!(
            ", crop {} {} {}x{}",
            crop.x, crop.y, crop.width, crop.height
        )
    })
}

/// The speed, when it is not the source's own, and nothing at all when it is.
///
/// Silent on the ordinary clip for the same reason [`cropped`] is: `1×` on
/// every line of every description would bury the one shot where the answer is
/// worth reading. And it is worth reading — a slot holding twice its own length
/// of footage is invisible in a frame count.
fn retimed(speed: Speed) -> String {
    if speed.is_normal() {
        return String::new();
    }
    format!(", {speed}")
}

/// What a document's `fit` value is called, in the words the format uses.
const fn fit(fit: Fit) -> &'static str {
    match fit {
        Fit::Fit => "fit",
        Fit::Fill => "fill",
        Fit::Native => "native",
    }
}

/// One shape in a line: what it is, how big, and the two colours it is drawn
/// in — named only when there is one, so an absent fill reads as absent rather
/// than as a colour nobody chose.
fn drawn(shape: &Shape) -> String {
    let outline = match shape.geometry {
        Geometry::Rectangle { radius, .. } if radius > 0.0 => "rounded rect",
        Geometry::Rectangle { .. } => "rect",
        Geometry::Ellipse { .. } => "ellipse",
    };
    let mut line = format!(
        "{outline} {}×{}",
        shape.geometry.width(),
        shape.geometry.height()
    );
    if let Some(fill) = shape.fill {
        line.push_str(&format!(", fill {fill}"));
    }
    if let Some(stroke) = shape.stroke {
        line.push_str(&format!(", border {stroke}"));
    }
    line
}

/// What a document's asset `kind` is called, in the words the format uses.
const fn kind(kind: AssetKind) -> &'static str {
    match kind {
        AssetKind::Video => "video",
        AssetKind::Image => "image",
        AssetKind::Audio => "audio",
        AssetKind::Text => "text",
        AssetKind::Color => "color",
        AssetKind::Shape => "shape",
        AssetKind::GeneratedVideo => "generated_video",
        AssetKind::GeneratedAudio => "generated_audio",
        AssetKind::SynthAudio => "synth_audio",
    }
}

/// What a document's `easing` value is called, in the words the format uses.
const fn easing(easing: Easing) -> &'static str {
    match easing {
        Easing::Linear => "linear",
        Easing::EaseIn => "ease_in",
        Easing::EaseOut => "ease_out",
        Easing::EaseInOut => "ease_in_out",
        Easing::Hold => "hold",
    }
}
