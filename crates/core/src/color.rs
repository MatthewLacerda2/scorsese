//! Colour as a document writes it.
//!
//! Stored as a hex string rather than four numbers, because `#ffcc00` is how
//! every tool a person has ever used writes a colour, and a document meant to
//! be hand-authored should not ask anyone to count commas.
//!
//! Its own module rather than text's, because more than one thing in a project
//! is a colour: the glyphs of a [`crate::TextStyle`], and the whole of a
//! [`crate::AssetKind::Color`] asset. One notation, read one way, wherever a
//! document names a colour.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// An 8-bit-per-channel colour with **straight** alpha — the same convention
/// the compositor's frames use, so no channel is scaled on the way through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Rgba {
    /// Red, `0` to `255`.
    pub r: u8,
    /// Green, `0` to `255`.
    pub g: u8,
    /// Blue, `0` to `255`.
    pub b: u8,
    /// Opacity, `0` invisible and `255` solid. Multiplied by the clip's own
    /// `opacity` when the layer is composited, rather than replacing it.
    pub a: u8,
}

impl Rgba {
    /// Opaque white — what a caption is unless someone says otherwise, since
    /// it is the colour that reads over the widest range of footage.
    pub const WHITE: Self = Self::opaque(255, 255, 255);

    /// Opaque black — what a picture is where nothing covers it.
    pub const BLACK: Self = Self::opaque(0, 0, 0);

    /// A colour with the alpha spelled out.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// A fully solid colour.
    pub const fn opaque(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, u8::MAX)
    }

    /// The channels in the order a straight-RGBA frame stores them.
    pub const fn channels(self) -> [u8; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

impl Default for Rgba {
    fn default() -> Self {
        Self::WHITE
    }
}

impl FromStr for Rgba {
    type Err = ColorError;

    /// Reads `#rrggbb` or `#rrggbbaa`. The `#` is optional and the digits may
    /// be either case; anything else is refused rather than guessed at.
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let digits = text.strip_prefix('#').unwrap_or(text);
        let malformed = || ColorError::Malformed(text.to_owned());
        if !matches!(digits.len(), 6 | 8) || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(malformed());
        }
        let channel =
            |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).map_err(|_| malformed());
        Ok(Self {
            r: channel(0)?,
            g: channel(2)?,
            b: channel(4)?,
            a: if digits.len() == 8 {
                channel(6)?
            } else {
                u8::MAX
            },
        })
    }
}

impl fmt::Display for Rgba {
    /// Writes the shorter of the two forms: the alpha only appears when it is
    /// not solid, so a document does not fill up with `ff` nobody chose.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{:02x}{:02x}{:02x}", self.r, self.g, self.b)?;
        if self.a != u8::MAX {
            write!(f, "{:02x}", self.a)?;
        }
        Ok(())
    }
}

impl TryFrom<String> for Rgba {
    type Error = ColorError;

    fn try_from(text: String) -> Result<Self, Self::Error> {
        text.parse()
    }
}

impl From<Rgba> for String {
    fn from(color: Rgba) -> Self {
        color.to_string()
    }
}

/// Why a string is not a colour.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ColorError {
    /// Not the right number of hex digits, or not hex digits at all. Refused
    /// at the parse rather than defaulted to black: a colour nobody can read
    /// back is a title that silently comes out the wrong colour.
    #[error("`{0}` is not a colour — write `#rrggbb`, or `#rrggbbaa` for one you can see through")]
    Malformed(
        /// The text as written.
        String,
    ),
}
