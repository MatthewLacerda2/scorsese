//! Writing the catalogue blob.
//!
//! **The format is specified once, in the compositor's `icon::catalogue`
//! module doc, and this file is the other half of it.** Change one and the
//! other stops reading what it wrote — which the compositor's own test suite
//! catches, because it reads every icon out of the committed blob and draws it.
//!
//! Everything here is little-endian and length-prefixed, and the records are
//! written in name order so the reader can binary-search them.

use crate::geometry::Command;
use crate::svg::Drawing;

/// What the reader checks before it believes a byte of the rest.
pub const MAGIC: &[u8] = b"SCORICON";

/// The format's own version, which is not Lucide's. It moves when the layout
/// below changes, so a blob written by an older tool is refused rather than
/// misread. Moved to 2 when aliases joined a record.
pub const FORMAT: u8 = 2;

/// One icon, as the tool has it before it is written down.
pub struct Entry {
    /// The name a document will one day write, which is the file's stem.
    pub name: String,
    /// Upstream's `tags`, in upstream's order.
    pub tags: Vec<String>,
    /// Upstream's `categories`, in upstream's order.
    pub categories: Vec<String>,
    /// The names this icon used to answer to upstream, in upstream's order.
    /// Findable, never writable — see the compositor's `icon::search`.
    pub aliases: Vec<String>,
    /// The two contour lists.
    pub drawing: Drawing,
}

/// The whole catalogue as bytes. `icons` must already be sorted by name.
pub fn encode(version: &str, icons: &[Entry]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.push(FORMAT);
    short(&mut out, version.as_bytes(), "the Lucide version")?;
    let count = u32::try_from(icons.len()).map_err(|_| "too many icons".to_string())?;
    out.extend_from_slice(&count.to_le_bytes());

    for icon in icons {
        short(&mut out, icon.name.as_bytes(), &icon.name)?;
        words(&mut out, &icon.tags, &icon.name)?;
        words(&mut out, &icon.categories, &icon.name)?;
        words(&mut out, &icon.aliases, &icon.name)?;
        block(&mut out, &icon.drawing.stroked);
        block(&mut out, &icon.drawing.filled);
    }
    Ok(out)
}

/// A byte string of at most 255 bytes, length first.
fn short(out: &mut Vec<u8>, bytes: &[u8], what: &str) -> Result<(), String> {
    let length = u8::try_from(bytes.len()).map_err(|_| format!("{what}: over 255 bytes"))?;
    out.push(length);
    out.extend_from_slice(bytes);
    Ok(())
}

/// A list of at most 255 short strings, count first.
fn words(out: &mut Vec<u8>, words: &[String], what: &str) -> Result<(), String> {
    let count = u8::try_from(words.len()).map_err(|_| format!("{what}: over 255 words"))?;
    out.push(count);
    for word in words {
        short(out, word.as_bytes(), what)?;
    }
    Ok(())
}

/// One contour list: its length in bytes, then the commands.
///
/// Length-prefixed so the reader can index an icon's geometry and decode it
/// only when something actually draws it — the catalogue is read for its names
/// and tags far more often than for its curves.
fn block(out: &mut Vec<u8>, commands: &[Command]) {
    let mut bytes = Vec::new();
    for command in commands {
        match *command {
            Command::Move(to) => {
                bytes.push(1);
                point(&mut bytes, to);
            }
            Command::Line(to) => {
                bytes.push(2);
                point(&mut bytes, to);
            }
            Command::Cubic(first, second, to) => {
                bytes.push(3);
                point(&mut bytes, first);
                point(&mut bytes, second);
                point(&mut bytes, to);
            }
            Command::Quad(control, to) => {
                bytes.push(4);
                point(&mut bytes, control);
                point(&mut bytes, to);
            }
            Command::Close => bytes.push(5),
        }
    }
    let length = bytes.len() as u32;
    out.extend_from_slice(&length.to_le_bytes());
    out.extend_from_slice(&bytes);
}

/// A coordinate pair, as two `f32`s in the 24-unit space.
///
/// `f32` rather than a fixed point: the conversion is arithmetic the tool does
/// once, and a rounding it introduced would be a rounding no test could
/// distinguish from a bad curve. Half a megabyte is not worth being unsure.
fn point(bytes: &mut Vec<u8>, point: (f64, f64)) {
    bytes.extend_from_slice(&(point.0 as f32).to_le_bytes());
    bytes.extend_from_slice(&(point.1 as f32).to_le_bytes());
}
