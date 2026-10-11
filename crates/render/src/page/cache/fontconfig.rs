//! The shipped faces, as the browser sees them and as a slot is named by.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use scorsese_core::{CACHE_DIR, hash_bytes};

use super::PAGES;

/// The shipped faces ([`super::super::fonts`]) written out where the browser can find them, with a
/// fontconfig file listing **only** them — so a page that names a font it was
/// not given falls back to the same face on every machine, never to whatever
/// the host has installed (#606). Returns that file.
///
/// Written once per project, and whole or not at all ([`super::write_whole`]):
/// a face whose file is already there at the right length, and a fontconfig
/// file that already says what it would, are left alone — a browser starting
/// on another thread never reads one cut short (#1085).
pub(crate) fn fonts(project_root: &Path) -> std::io::Result<PathBuf> {
    let folder = project_root.join(CACHE_DIR).join(PAGES).join("fonts");
    std::fs::create_dir_all(&folder)?;
    for face in super::super::fonts::faces() {
        let file = folder.join(&face.file);
        let current = std::fs::metadata(&file).map(|m| m.len()).ok();
        if current != Some(face.bytes.len() as u64) {
            super::write_whole(&file, face.bytes)?;
        }
    }
    let conf = folder.join("fonts.conf");
    let text = format!(
        "<?xml version=\"1.0\"?>\n<fontconfig>\n  <dir>{}</dir>\n  <cachedir>{}</cachedir>\n</fontconfig>\n",
        escaped(&folder),
        escaped(&folder.join(".cache"))
    );
    if std::fs::read(&conf).ok().as_deref() != Some(text.as_bytes()) {
        super::write_whole(&conf, text.as_bytes())?;
    }
    Ok(conf)
}

/// `path` as XML text: a project in `Tom & Jerry.scor` is a folder like any
/// other.
fn escaped(path: &Path) -> String {
    let mut text = String::new();
    for c in path.display().to_string().chars() {
        match c {
            '&' => text.push_str("&amp;"),
            '<' => text.push_str("&lt;"),
            '>' => text.push_str("&gt;"),
            '"' => text.push_str("&quot;"),
            '\'' => text.push_str("&apos;"),
            c => text.push(c),
        }
    }
    text
}

/// One hash over every shipped face: a build that ships different fonts draws
/// different pages.
pub(super) fn fonts_hash() -> &'static str {
    static HASH: OnceLock<String> = OnceLock::new();
    HASH.get_or_init(|| {
        let digests: Vec<String> = super::super::fonts::faces()
            .iter()
            .map(|face| format!("{}:{}", face.file, hash_bytes(face.bytes)))
            .collect();
        hash_bytes(digests.join("\n").as_bytes())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_browser_gets_every_shipped_face_and_a_fontconfig_naming_only_them() {
        let root = crate::scratch::fresh("scorsese-fonts", "");
        let conf = fonts(&root).unwrap();
        let folder = conf.parent().unwrap().to_path_buf();
        assert_eq!(folder, root.join("cache/pages/fonts"));
        let text = std::fs::read_to_string(&conf).unwrap();
        assert!(
            text.contains(&format!("<dir>{}</dir>", folder.display())),
            "{text}"
        );
        let written = std::fs::metadata(&conf).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        fonts(&root).unwrap();
        assert_eq!(
            std::fs::metadata(&conf).unwrap().modified().unwrap(),
            written,
            "a fontconfig file already right is left alone"
        );
        let inter = folder.join("inter.ttf");
        assert_eq!(
            std::fs::read(&inter).unwrap(),
            super::super::super::fonts::bytes("inter.ttf").unwrap()
        );
        // A face cut short — an interrupted write — is written again.
        std::fs::write(&inter, b"short").unwrap();
        fonts(&root).unwrap();
        assert_eq!(
            std::fs::metadata(&inter).unwrap().len(),
            super::super::super::fonts::bytes("inter.ttf")
                .unwrap()
                .len() as u64
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_folder_with_markup_in_its_name_is_still_a_folder() {
        assert_eq!(
            escaped(Path::new("/p/Tom & Jerry's <\"best\">.scor")),
            "/p/Tom &amp; Jerry&apos;s &lt;&quot;best&quot;&gt;.scor"
        );
        let root = crate::scratch::fresh("scorsese-fonts & ", "");
        let text = std::fs::read_to_string(fonts(&root).unwrap()).unwrap();
        assert!(text.contains("scorsese-fonts &amp; "), "{text}");
        assert!(!text.contains("scorsese-fonts & "), "{text}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_fonts_are_named_by_a_hash_of_their_bytes() {
        let hash = fonts_hash();
        assert_eq!(hash.len(), 64);
        assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_ne!(hash, hash_bytes(b""));
    }
}
