//! # scorsese-icon-vendor — the vendored Lucide tree, converted
//!
//! Reads `crates/compositor/icons/lucide/` and writes the `catalogue.bin` the
//! compositor compiles in. Run by hand when the pinned version moves, and at no
//! other time:
//!
//! ```text
//! cargo run -p scorsese-icon-vendor            # rewrite the blob
//! cargo run -p scorsese-icon-vendor -- --check # is the committed blob current?
//! ```
//!
//! **Why a tool and not a `build.rs`.** A build script would re-read 1767 SVGs
//! on a clean build of the crate everything downstream depends on, to produce a
//! file that changes about once a year. The blob is committed instead, which
//! also makes the conversion reviewable: a bump is a diff of the vendored tree
//! *and* a diff of the bytes it produces, and `--check` is how anyone confirms
//! the second follows from the first.
//!
//! It converts everything or nothing. One unreadable icon stops the run and
//! names itself, because the alternative is a catalogue quietly missing an
//! entry that upstream has. That strictness is the whole value of converting
//! ahead of time: the failure lands on the person doing the vendoring, who can
//! see both versions, rather than on a render.
//!
//! Boundary: nothing here is compiled into scorsese. It depends on no crate of
//! this workspace, and no crate of this workspace depends on it.

pub mod blob;
pub mod geometry;
pub mod path;
pub mod svg;

use std::path::Path;

use blob::Entry;

/// The blob's name inside the icons directory.
pub const CATALOGUE: &str = "catalogue.bin";

/// Converts the tree under `root`, and either writes the blob or checks the
/// committed one against it.
///
/// `root` is the icons directory: `VERSION`, `lucide/` and the blob itself.
pub fn run(root: &Path, checking: bool) -> Result<String, String> {
    let version = read(&root.join("VERSION"))?.trim().to_string();
    if version.is_empty() {
        return Err("VERSION is empty; it records the pinned Lucide release".to_string());
    }

    let icons = catalogue(&root.join("lucide"))?;
    let bytes = blob::encode(&version, &icons)?;
    let at = root.join(CATALOGUE);

    if checking {
        let committed = std::fs::read(&at).map_err(|error| format!("{}: {error}", at.display()))?;
        if committed != bytes {
            return Err(format!(
                "{CATALOGUE} is not what the vendored tree converts to -- \
                 re-run without --check and commit the result"
            ));
        }
        return Ok(format!(
            "{CATALOGUE} is current -- {} icons, Lucide {version}",
            icons.len()
        ));
    }

    std::fs::write(&at, &bytes).map_err(|error| format!("{}: {error}", at.display()))?;
    Ok(format!(
        "wrote {CATALOGUE} -- {} icons, Lucide {version}, {} KiB",
        icons.len(),
        bytes.len() / 1024
    ))
}

/// Every icon in a vendored directory, in name order.
///
/// The `.svg` files decide what the catalogue holds; a `.json` beside each one
/// carries the words it is found by. A missing or unreadable metadata file is
/// an error rather than an icon with no tags — search is what the metadata is
/// carried for, and an icon nobody can find is half vendored.
pub fn catalogue(from: &Path) -> Result<Vec<Entry>, String> {
    let listing =
        std::fs::read_dir(from).map_err(|error| format!("{}: {error}", from.display()))?;
    let files: Vec<_> = listing
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().is_some_and(|kind| kind == "svg"))
        .collect();
    if files.is_empty() {
        return Err(format!("{}: no icons here", from.display()));
    }

    let mut icons: Vec<Entry> = files
        .iter()
        .map(|file| entry(file).map_err(|error| format!("{}: {error}", file.display())))
        .collect::<Result<_, _>>()?;
    // By name, which is not the same order as by file name: `arrow-up-right`
    // sorts after `arrow-up`, while `arrow-up-right.svg` sorts before
    // `arrow-up.svg`. The reader binary-searches the names, so it is the names
    // that have to be in order.
    icons.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(icons)
}

/// One icon, from its `.svg` and the `.json` beside it.
fn entry(file: &Path) -> Result<Entry, String> {
    let name = file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or("an unreadable file name")?
        .to_string();
    let drawing = svg::parse(&read(file)?)?;
    let metadata = read(&file.with_extension("json"))?;
    let metadata: serde_json::Value =
        serde_json::from_str(&metadata).map_err(|error| format!("its metadata: {error}"))?;
    Ok(Entry {
        name,
        tags: words(&metadata, "tags")?,
        categories: words(&metadata, "categories")?,
        drawing,
    })
}

/// One list of words out of an icon's metadata. Absent is empty; present and
/// not a list of strings is an error, because that is upstream changing shape.
fn words(metadata: &serde_json::Value, field: &str) -> Result<Vec<String>, String> {
    let Some(value) = metadata.get(field) else {
        return Ok(Vec::new());
    };
    value
        .as_array()
        .ok_or_else(|| format!("its '{field}' is not a list"))?
        .iter()
        .map(|word| {
            word.as_str()
                .map(str::to_string)
                .ok_or_else(|| format!("its '{field}' holds something that is not a word"))
        })
        .collect()
}

fn read(file: &Path) -> Result<String, String> {
    std::fs::read_to_string(file).map_err(|error| format!("{}: {error}", file.display()))
}
