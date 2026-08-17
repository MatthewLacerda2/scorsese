//! The compiled-in blob, and the format it is written in.
//!
//! **The catalogue is data, not code.** Seventeen hundred entries spelled out
//! as Rust would be a quarter of a megabyte of source that nobody edits and the
//! size gate would have to be argued with — so the entries are bytes, produced
//! by `tools/icons` at vendor time and pulled in here with `include_bytes!`,
//! exactly as `text::font::shipped` pulls in a face. Nothing is read
//! from disk at runtime, which is what keeps the crate's "no file I/O" boundary
//! true.
//!
//! ## The format
//!
//! Little-endian throughout, every string and every list length-prefixed, and
//! the records in ascending name order so a lookup is a binary search. The
//! writer is `tools/icons/src/blob.rs`; the two halves are specified here.
//!
//! ```text
//! header   "SCORICON"  8 bytes
//!          format      u8      the layout below, currently 2
//!          version     short   the Lucide release the tree was vendored from
//!          count       u32     how many records follow
//!
//! record   name        short
//!          tags        words
//!          categories  words
//!          aliases     words   names upstream retired for this one
//!          stroked     block   contours drawn with a line
//!          filled      block   contours drawn solid — a handful of dots
//!
//! short    u8 length, then that many bytes of UTF-8
//! words    u8 count, then that many shorts
//! block    u32 length in bytes, then that many bytes of commands
//!
//! command  1 move   x y            all coordinates f32, in the 24-unit space
//!          2 line   x y
//!          3 cubic  x1 y1 x2 y2 x y
//!          4 quad   x1 y1 x y
//!          5 close
//! ```
//!
//! A blob that does not read is an **empty** catalogue rather than a panic —
//! there is no drawing to be done from bytes nobody can parse, and a render
//! that stopped dead would be a worse answer than an icon that is missing. The
//! case is not left to chance: the test suite draws every entry, so a blob that
//! failed to parse is a suite that fails immediately rather than a build that
//! ships silently short.

use std::sync::OnceLock;

use super::Icon;

/// The catalogue itself, produced by `tools/icons` from `icons/lucide/`.
static BLOB: &[u8] = include_bytes!("../../icons/catalogue.bin");

/// What the first bytes say, when the blob is one of ours.
const MAGIC: &[u8] = b"SCORICON";

/// The only layout this reader understands. A blob written before aliases
/// joined a record said 1, and is refused rather than read short.
const FORMAT: u8 = 2;

/// The parsed index: the Lucide release, and every icon in name order.
static CATALOGUE: OnceLock<(&'static str, Vec<Icon>)> = OnceLock::new();

fn catalogue() -> &'static (&'static str, Vec<Icon>) {
    CATALOGUE.get_or_init(|| parse(BLOB).unwrap_or(("", Vec::new())))
}

/// Every icon, in ascending name order.
pub(super) fn all() -> &'static [Icon] {
    &catalogue().1
}

/// The Lucide release the vendored tree came from, empty for a blob that did
/// not read.
pub(super) fn version() -> &'static str {
    catalogue().0
}

/// Reads the whole blob, or nothing at all.
fn parse(blob: &'static [u8]) -> Option<(&'static str, Vec<Icon>)> {
    let mut reader = Reader { blob, at: 0 };
    if reader.take(MAGIC.len())? != MAGIC || reader.byte()? != FORMAT {
        return None;
    }
    let version = reader.short()?;
    let count = reader.number()?;

    let mut icons: Vec<Icon> = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let icon = Icon {
            name: reader.short()?,
            tags: reader.words()?,
            categories: reader.words()?,
            aliases: reader.words()?,
            stroked: reader.block()?,
            filled: reader.block()?,
        };
        // Ascending and unique, which is what makes a lookup a binary search.
        // Checked rather than trusted: the writer sorts, and a reader that
        // silently stopped finding half the set would be a hard bug to see.
        if icons.last().is_some_and(|last| last.name >= icon.name) {
            return None;
        }
        icons.push(icon);
    }
    (reader.at == blob.len()).then_some((version, icons))
}

/// A cursor over the blob. Every read is bounds-checked and answers `None`
/// past the end, so a truncated blob is caught rather than indexed into.
struct Reader {
    blob: &'static [u8],
    at: usize,
}

impl Reader {
    fn take(&mut self, length: usize) -> Option<&'static [u8]> {
        let bytes = self.blob.get(self.at..self.at.checked_add(length)?)?;
        self.at += length;
        Some(bytes)
    }

    fn byte(&mut self) -> Option<u8> {
        self.take(1).map(|bytes| bytes[0])
    }

    fn number(&mut self) -> Option<u32> {
        let bytes: [u8; 4] = self.take(4)?.try_into().ok()?;
        Some(u32::from_le_bytes(bytes))
    }

    fn short(&mut self) -> Option<&'static str> {
        let length = self.byte()? as usize;
        std::str::from_utf8(self.take(length)?).ok()
    }

    fn words(&mut self) -> Option<Vec<&'static str>> {
        let count = self.byte()?;
        (0..count).map(|_| self.short()).collect()
    }

    fn block(&mut self) -> Option<&'static [u8]> {
        let length = self.number()? as usize;
        self.take(length)
    }
}
