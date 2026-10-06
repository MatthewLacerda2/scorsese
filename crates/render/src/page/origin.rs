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

use std::path::{Component, Path, PathBuf};

/// The origin a page is served from. Not a real host: nothing resolves it, and
/// no request to it ever leaves the process.
pub(crate) const ORIGIN: &str = "https://page.scorsese";

/// The origin the shipped libraries are served from. A host of its own rather
/// than a folder of the page's, so no project directory can ever shadow one.
pub const SHIPPED_ORIGIN: &str = "https://lib.scorsese";

/// The libraries every page may load, by the file name they are served at.
/// The shipped fonts are served beside them, under `fonts/` ([`super::fonts`]).
///
/// anime.js because #606 measured it under the clock: a library that drives
/// itself from `requestAnimationFrame` is seekable by construction. MIT, and
/// its licence is vendored beside it.
const SHIPPED: &[(&str, &[u8])] = &[
    ("anime.min.js", include_bytes!("shipped/anime.min.js")),
    (
        "anime.LICENSE.txt",
        include_bytes!("shipped/anime.LICENSE.txt"),
    ),
];

/// What a request is answered with.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    /// A file of the project, by its path from the project root.
    File { path: String, body: Vec<u8> },
    /// A library this build ships.
    Shipped { body: &'static [u8] },
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

/// Answers one request the page made.
pub(crate) fn answer(url: &str, project_root: &Path) -> Answer {
    if let Some(name) = url
        .strip_prefix(SHIPPED_ORIGIN)
        .and_then(|rest| rest.strip_prefix('/'))
    {
        let name = name.split(['?', '#']).next().unwrap_or_default();
        if let Some(body) = name.strip_prefix("fonts/").and_then(super::fonts::bytes) {
            return Answer::Shipped { body };
        }
        return SHIPPED
            .iter()
            .find(|(shipped, _)| *shipped == name)
            .map_or(Answer::Refused, |(_, body)| Answer::Shipped { body });
    }
    let Some(path) = url
        .strip_prefix(ORIGIN)
        .and_then(|rest| rest.strip_prefix('/'))
        .and_then(project_path)
    else {
        return Answer::Refused;
    };
    let Some(file) = inside(project_root, &path) else {
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

/// The file at `path` under the root, if it is really under it — a symbolic
/// link inside the project pointing out of it is outside it.
fn inside(project_root: &Path, path: &str) -> Option<PathBuf> {
    let file = project_root.join(path);
    let Ok(real) = file.canonicalize() else {
        // Nothing there to follow; whether it is missing is the caller's call.
        return Some(file);
    };
    let root = project_root.canonicalize().ok()?;
    real.starts_with(&root).then_some(real)
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
mod tests {
    use super::*;

    fn project() -> tempfile_free::Dir {
        let dir = tempfile_free::Dir::new("origin");
        std::fs::create_dir_all(dir.0.join("pages")).unwrap();
        std::fs::create_dir_all(dir.0.join("assets")).unwrap();
        std::fs::write(dir.0.join("pages/title.html"), "<p>hi</p>").unwrap();
        std::fs::write(dir.0.join("assets/a b.png"), [1, 2, 3]).unwrap();
        dir
    }

    /// A scratch directory removed when dropped, without a crate for it.
    mod tempfile_free {
        pub(super) struct Dir(pub(super) std::path::PathBuf);
        impl Dir {
            pub(super) fn new(name: &str) -> Self {
                let path = std::env::temp_dir().join(format!(
                    "scorsese-{name}-{}-{:?}",
                    std::process::id(),
                    std::thread::current().id()
                ));
                let _ = std::fs::remove_dir_all(&path);
                Self(path)
            }
        }
        impl Drop for Dir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    #[test]
    fn a_project_file_is_served_by_its_path_from_the_root() {
        let dir = project();
        assert_eq!(
            answer("https://page.scorsese/pages/title.html?x=1#top", &dir.0),
            Answer::File {
                path: "pages/title.html".into(),
                body: b"<p>hi</p>".to_vec()
            }
        );
        assert_eq!(
            answer("https://page.scorsese/assets/a%20b.png", &dir.0),
            Answer::File {
                path: "assets/a b.png".into(),
                body: vec![1, 2, 3]
            }
        );
    }

    #[test]
    fn a_project_path_is_served_at_the_url_that_answers_with_it() {
        let dir = project();
        let url = url_of("pages/title.html");
        assert_eq!(url, "https://page.scorsese/pages/title.html");
        assert!(
            matches!(answer(&url, &dir.0), Answer::File { path, .. } if path == "pages/title.html")
        );
    }

    #[test]
    fn a_missing_file_is_missing_and_everything_outside_is_refused() {
        let dir = project();
        assert_eq!(
            answer("https://page.scorsese/assets/gone.png", &dir.0),
            Answer::Missing {
                path: "assets/gone.png".into()
            }
        );
        for url in [
            "https://fonts.googleapis.com/css?family=Inter",
            "http://page.scorsese/pages/title.html",
            "https://page.scorsese.evil/pages/title.html",
            "https://page.scorsese/../etc/passwd",
            "https://page.scorsese/%2e%2e/etc/passwd",
            "https://page.scorsese/",
            "https://lib.scorsese/nothing.js",
        ] {
            assert_eq!(answer(url, &dir.0), Answer::Refused, "{url}");
        }
    }

    #[test]
    fn the_shipped_libraries_come_from_their_own_origin() {
        let dir = project();
        let Answer::Shipped { body } = answer("https://lib.scorsese/anime.min.js", &dir.0) else {
            panic!("anime.js is shipped");
        };
        assert!(body.starts_with(b"/*\n * anime.js v3.2.2"));
        assert!(matches!(
            answer("https://lib.scorsese/fonts/inter.ttf", &dir.0),
            Answer::Shipped { .. }
        ));
    }

    #[test]
    fn every_kind_of_file_a_page_loads_has_its_own_type() {
        // Each one, because a stylesheet or a module script served as anything
        // else is refused by the browser, and that refusal is silent.
        for (file, mime) in [
            ("a.html", "text/html; charset=utf-8"),
            ("a.htm", "text/html; charset=utf-8"),
            ("a.css", "text/css; charset=utf-8"),
            ("a.js", "text/javascript; charset=utf-8"),
            ("a.mjs", "text/javascript; charset=utf-8"),
            ("a.json", "application/json"),
            ("a.svg", "image/svg+xml"),
            ("a.png", "image/png"),
            ("a.jpg", "image/jpeg"),
            ("a.jpeg", "image/jpeg"),
            ("a.gif", "image/gif"),
            ("a.webp", "image/webp"),
            ("a.avif", "image/avif"),
            ("a.woff", "font/woff"),
            ("a.woff2", "font/woff2"),
            ("a.ttf", "font/ttf"),
            ("a.otf", "font/otf"),
            ("a.txt", "text/plain; charset=utf-8"),
        ] {
            assert_eq!(Answer::mime(&format!("https://x/{file}")), mime, "{file}");
        }
    }

    #[test]
    fn a_response_says_what_it_is_by_extension() {
        assert_eq!(
            Answer::mime("https://x/a.MJS?v=2"),
            "text/javascript; charset=utf-8"
        );
        assert_eq!(Answer::mime("https://x/f.woff2"), "font/woff2");
        assert_eq!(Answer::mime("https://x/blob"), "application/octet-stream");
    }
}
