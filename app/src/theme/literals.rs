//! The app's own rule, held (#1228): every colour and every font size is named
//! in `theme`, so a panel that spells `Color32::…` or a numeric
//! `FontId::proportional(…)` / `FontId::monospace(…)` is a bug.
//!
//! Clippy cannot ban a constant such as `Color32::WHITE`, so this is a scan of
//! the source. Type *names* (`fn tint(…) -> Color32`) are fine; a path into
//! `Color32::` is what spells a colour.

use std::path::Path;

fn scan(dir: &Path, found: &mut Vec<String>) {
    for entry in std::fs::read_dir(dir).expect("source folder") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "theme") {
                continue;
            }
            scan(&path, found);
        } else if path.extension().is_some_and(|e| e == "rs") {
            let text = std::fs::read_to_string(&path).expect("source file");
            for (number, line) in text.lines().enumerate() {
                let code = line.split("//").next().unwrap_or("");
                if is_literal(code) {
                    found.push(format!(
                        "{}:{}: {}",
                        path.display(),
                        number + 1,
                        line.trim()
                    ));
                }
            }
        }
    }
}

/// A line that spells a colour or a number-sized font.
fn is_literal(code: &str) -> bool {
    code.contains("Color32::")
        || ["FontId::proportional(", "FontId::monospace("]
            .iter()
            .any(|call| {
                code.match_indices(call).any(|(at, _)| {
                    code[at + call.len()..]
                        .chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_digit())
                })
            })
}

/// Outside `theme`, nothing spells a colour or a font size.
#[test]
fn no_panel_spells_a_colour_or_a_size() {
    let mut found = Vec::new();
    scan(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut found,
    );
    assert!(
        found.is_empty(),
        "name these in `theme`:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_scan_flags_a_colour_and_a_numeric_size_and_passes_a_named_one() {
    assert!(is_literal("let c = Color32::WHITE;"));
    assert!(is_literal("FontId::proportional(13.0)"));
    assert!(is_literal("FontId::monospace(9.5)"));
    assert!(!is_literal("FontId::proportional(size)"));
    assert!(!is_literal("fn tint() -> Color32 {"));
}
