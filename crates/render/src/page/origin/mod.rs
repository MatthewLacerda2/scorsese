//! Where a page is served from: an origin that exists only inside a capture.
//!
//! The browser is pointed at `https://page.scorsese/pages/title.html`, and every
//! request it makes is paused (the `Fetch` domain) and answered here, from the
//! project directory. So the page's relative links resolve against the project
//! root exactly as its `path` does — `../assets/photo.png` from `pages/` is the
//! project's `assets/photo.png` — and there is no `file://` with its quirks (an
//! ES module or a font refuses to load from one).
//!
//! **The offline rule and the allow-list are this one function.** A request for
//! a file inside the project is answered with it; a request for one of the
//! libraries this build ships is answered from the binary; everything else —
//! any other host, a path that leaves the project — is refused, and the refusal
//! becomes a warning on the render, never a silent miss.
//!
//! **A link out of the project is outside it**, unless it lands under a folder
//! the caller named ([`super::capture_following`]). Locally nobody names one,
//! so a `.scor` folder's link to `/etc/passwd` is refused; the web app names
//! the owner's library, because it lays every media file out as a link into
//! it (#857) — and a link from there to `/etc/passwd` is refused all the same,
//! since it is where a link really leads that is checked.

use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

use super::icons::{self, Served};

/// The origin a page is served from. Not a real host: nothing resolves it, and
/// no request to it ever leaves the process.
pub(crate) const ORIGIN: &str = "https://page.scorsese";

/// The origin the shipped libraries are served from. A host of its own rather
/// than a folder of the page's, so no project directory can ever shadow one.
pub const SHIPPED_ORIGIN: &str = "https://lib.scorsese";

/// The libraries every page may load, by the file name they are served at.
/// The shipped fonts are served beside them, under `fonts/` ([`super::fonts`]),
/// and the icon set under `icons/` ([`super::icons`]).
///
/// anime.js 3.2.2 because #606 measured it under the clock: a library that
/// drives itself from `requestAnimationFrame` is seekable by construction.
/// lottie-web 5.13.0 (#903), the full build (every renderer, and expressions,
/// which many published animations use), for the Lottie files `stock_import`
/// brings in: a page drives it from the clock with `goToAndStop`, never its
/// own playback. Both MIT, each licence vendored beside it. And scorsese's
/// own motion kit (#812), `kit.js`: the helpers every timed page was writing
/// for itself, on the page's seconds.
///
/// Every file served from here is part of the capture of a page that loads it
/// (`record`'s key), so changing one draws again only the pages that loaded
/// it — never every page, which is what [`super::PAGE_VERSION`] does.
const SHIPPED: &[(&str, &[u8])] = &[
    ("anime.min.js", include_bytes!("../shipped/anime.min.js")),
    (
        "anime.LICENSE.txt",
        include_bytes!("../shipped/anime.LICENSE.txt"),
    ),
    ("lottie.min.js", include_bytes!("../shipped/lottie.min.js")),
    (
        "lottie.LICENSE.txt",
        include_bytes!("../shipped/lottie.LICENSE.txt"),
    ),
    ("kit.js", include_bytes!("../shipped/kit.js")),
];

/// What a request for a shipped file is kept under in a capture's record:
/// its URL without query or fragment — a scheme no project path has, so the
/// two can never be mistaken for each other. `None` for any other URL.
pub(crate) fn record_key(url: &str) -> Option<String> {
    url.strip_prefix(SHIPPED_ORIGIN)?;
    url.split(['?', '#']).next().map(str::to_owned)
}

/// The hash of what this build serves at a shipped file's `key`
/// ([`record_key`]), or `None` when it serves nothing there — so a capture
/// that loaded a shipped file is fresh exactly while the file is unchanged,
/// and one refused a file this build now ships is stale.
pub(crate) fn shipped_hash(key: &str) -> Option<String> {
    match answer(key, Path::new(""), &[]) {
        Answer::Shipped { body } => Some(scorsese_core::hash_bytes(&body)),
        _ => None,
    }
}

/// What a request is answered with.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    /// A file of the project, by its path from the project root.
    File { path: String, body: Vec<u8> },
    /// A library this build ships — a file compiled in, or an icon written
    /// out on request.
    Shipped { body: Cow<'static, [u8]> },
    /// An icon the shipped set does not have, with the names it nearly was.
    UnknownIcon {
        name: String,
        nearest: Vec<&'static str>,
    },
    /// A path inside the project with no file at it.
    Missing { path: String },
    /// Anything outside the project — the internet, mostly.
    Refused,
}

impl Answer {
    /// What the response says it is. Pages are fussy about this for scripts
    /// loaded as modules and for stylesheets, so it is decided by extension
    /// rather than left to sniffing.
    pub(crate) fn mime(url: &str) -> &'static str {
        let path = url.split(['?', '#']).next().unwrap_or_default();
        let extension = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase());
        match extension.as_deref() {
            Some("html" | "htm") => "text/html; charset=utf-8",
            Some("css") => "text/css; charset=utf-8",
            Some("js" | "mjs") => "text/javascript; charset=utf-8",
            Some("json") => "application/json",
            Some("svg") => "image/svg+xml",
            Some("png") => "image/png",
            Some("jpg" | "jpeg") => "image/jpeg",
            Some("gif") => "image/gif",
            Some("webp") => "image/webp",
            Some("avif") => "image/avif",
            Some("woff") => "font/woff",
            Some("woff2") => "font/woff2",
            Some("ttf") => "font/ttf",
            Some("otf") => "font/otf",
            Some("txt") => "text/plain; charset=utf-8",
            _ => "application/octet-stream",
        }
    }
}

/// The URL a project path is served at.
pub(crate) fn url_of(path: &str) -> String {
    format!("{ORIGIN}/{path}")
}

/// Answers one request the page made, from the project at `project_root`,
/// following its links into the folders `follow` names and nowhere else.
pub(crate) fn answer(url: &str, project_root: &Path, follow: &[PathBuf]) -> Answer {
    if let Some(name) = url
        .strip_prefix(SHIPPED_ORIGIN)
        .and_then(|rest| rest.strip_prefix('/'))
    {
        let name = name.split(['?', '#']).next().unwrap_or_default();
        if let Some(body) = name.strip_prefix("fonts/").and_then(super::fonts::bytes) {
            return Answer::Shipped { body: body.into() };
        }
        if let Some(file) = name.strip_prefix("icons/") {
            return match icons::serve(file) {
                Served::Svg(svg) => Answer::Shipped {
                    body: svg.into_bytes().into(),
                },
                Served::Unknown { name, nearest } => Answer::UnknownIcon { name, nearest },
            };
        }
        return SHIPPED.iter().find(|(shipped, _)| *shipped == name).map_or(
            Answer::Refused,
            |(_, body)| Answer::Shipped {
                body: Cow::Borrowed(body),
            },
        );
    }
    let Some(path) = url
        .strip_prefix(ORIGIN)
        .and_then(|rest| rest.strip_prefix('/'))
        .and_then(project_path)
    else {
        return Answer::Refused;
    };
    let Some(file) = inside(project_root, follow, &path) else {
        return Answer::Refused;
    };
    match std::fs::read(&file) {
        Ok(body) => Answer::File { path, body },
        Err(_) => Answer::Missing { path },
    }
}

/// The project path a URL's path names: query and fragment dropped, escapes
/// undone, and nothing that climbs or is empty. The browser has already folded
/// `..` away before it asks, so one arriving here is refused, not resolved.
fn project_path(rest: &str) -> Option<String> {
    let path = rest.split(['?', '#']).next()?;
    let decoded = percent_decode(path)?;
    let fine = !decoded.is_empty()
        && Path::new(&decoded)
            .components()
            .all(|part| matches!(part, Component::Normal(_)));
    fine.then_some(decoded)
}

/// The file at `path` under the root, if it really is under it or under one of
/// the folders `follow` names — a symbolic link inside the project pointing
/// anywhere else is outside it.
fn inside(project_root: &Path, follow: &[PathBuf], path: &str) -> Option<PathBuf> {
    let file = project_root.join(path);
    let Ok(real) = file.canonicalize() else {
        // Nothing there to follow; whether it is missing is the caller's call.
        return Some(file);
    };
    std::iter::once(project_root)
        .chain(follow.iter().map(PathBuf::as_path))
        .filter_map(|root| root.canonicalize().ok())
        .any(|root| real.starts_with(root))
        .then_some(real)
}

/// `%20` and friends back to bytes. `None` for a malformed escape or a result
/// that is not UTF-8, which no file of the project could be named.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'%' {
            let hex = text.get(at + 1..at + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            at += 3;
        } else {
            out.push(bytes[at]);
            at += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests;
