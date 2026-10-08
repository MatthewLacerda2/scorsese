//! What a page is answered with, and what a shipped file is recorded as.

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
        answer(
            "https://page.scorsese/pages/title.html?x=1#top",
            &dir.0,
            &[]
        ),
        Answer::File {
            path: "pages/title.html".into(),
            body: b"<p>hi</p>".to_vec()
        }
    );
    assert_eq!(
        answer("https://page.scorsese/assets/a%20b.png", &dir.0, &[]),
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
        matches!(answer(&url, &dir.0, &[]), Answer::File { path, .. } if path == "pages/title.html")
    );
}

#[test]
fn a_missing_file_is_missing_and_everything_outside_is_refused() {
    let dir = project();
    assert_eq!(
        answer("https://page.scorsese/assets/gone.png", &dir.0, &[]),
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
        assert_eq!(answer(url, &dir.0, &[]), Answer::Refused, "{url}");
    }
}

#[test]
fn a_link_out_of_the_project_is_refused_unless_it_lands_where_the_caller_said() {
    use std::os::unix::fs::symlink;
    let dir = project();
    let library = tempfile_free::Dir::new("origin-library");
    std::fs::create_dir_all(&library.0).unwrap();
    std::fs::write(library.0.join("photo.png"), [4, 5]).unwrap();
    symlink(library.0.join("photo.png"), dir.0.join("assets/photo.png")).unwrap();
    symlink("/etc/passwd", dir.0.join("assets/passwd")).unwrap();
    symlink("/etc/passwd", library.0.join("passwd")).unwrap();
    symlink(library.0.join("passwd"), dir.0.join("assets/via.txt")).unwrap();
    let photo = "https://page.scorsese/assets/photo.png";

    // Locally nobody names a folder: a `.scor` folder's links lead nowhere.
    assert_eq!(answer(photo, &dir.0, &[]), Answer::Refused);
    let named = [library.0.clone()];
    assert_eq!(
        answer(photo, &dir.0, &named),
        Answer::File {
            path: "assets/photo.png".into(),
            body: vec![4, 5]
        }
    );
    // Where a link really leads is what is checked, through every hop.
    for url in [
        "https://page.scorsese/assets/passwd",
        "https://page.scorsese/assets/via.txt",
    ] {
        assert_eq!(answer(url, &dir.0, &[]), Answer::Refused, "{url}");
        assert_eq!(answer(url, &dir.0, &named), Answer::Refused, "{url}");
    }
}

#[test]
fn the_shipped_libraries_come_from_their_own_origin() {
    let dir = project();
    let Answer::Shipped { body } = answer("https://lib.scorsese/anime.min.js", &dir.0, &[]) else {
        panic!("anime.js is shipped");
    };
    assert!(body.starts_with(b"/*\n * anime.js v3.2.2"));
    let Answer::Shipped { body } = answer("https://lib.scorsese/lottie.min.js", &dir.0, &[]) else {
        panic!("lottie-web is shipped");
    };
    let version = b"\"5.13.0\"";
    assert!(
        body.windows(version.len()).any(|at| at == version),
        "lottie-web 5.13.0"
    );
    assert!(matches!(
        answer("https://lib.scorsese/fonts/inter.ttf", &dir.0, &[]),
        Answer::Shipped { .. }
    ));
    let Answer::Shipped { body } = answer("https://lib.scorsese/icons/film.svg?v=1", &dir.0, &[])
    else {
        panic!("the icon set is shipped");
    };
    assert!(body.starts_with(b"<svg "));
}

#[test]
fn a_shipped_file_is_recorded_by_its_url_and_hashed_as_this_build_serves_it() {
    let key = record_key("https://lib.scorsese/lottie.min.js?v=2#x").unwrap();
    assert_eq!(key, "https://lib.scorsese/lottie.min.js");
    assert_eq!(
        shipped_hash(&key),
        Some(scorsese_core::hash_bytes(include_bytes!(
            "../shipped/lottie.min.js"
        )))
    );
    assert!(record_key("https://page.scorsese/pages/a.html").is_none());
    assert_eq!(shipped_hash("https://lib.scorsese/nothing.js"), None);
    assert_eq!(
        shipped_hash("https://lib.scorsese/icons/clapperbord.svg"),
        None
    );
    assert!(shipped_hash("https://lib.scorsese/icons/film.svg").is_some());
}

#[test]
fn an_icon_the_set_lacks_is_named_not_refused() {
    let dir = project();
    assert!(matches!(
        answer("https://lib.scorsese/icons/clapperbord.svg", &dir.0, &[]),
        Answer::UnknownIcon { name, nearest } if name == "clapperbord" && nearest.first() == Some(&"clapperboard")
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
