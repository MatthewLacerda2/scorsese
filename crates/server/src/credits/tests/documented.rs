//! `docs/web.md`'s multipliers, held to [`Product::percent`] (#1218).
//!
//! The page is where a reader learns what a user pays over cost, and it said
//! "cost + 10%" for a while after the code stopped meaning it. Nothing else
//! fails when the two disagree, so both places the page names them are read
//! here.

use crate::credits::Product;

const PAGE: &str = include_str!("../../../../../docs/web.md");

/// Every product, beside the *multipliers* table row that names it. A product
/// added without a row fails to compile here.
fn row(product: Product) -> &'static str {
    match product {
        Product::VeoShot => "Veo shot",
        Product::Still
        | Product::SpokenLine
        | Product::MusicTrack
        | Product::VoiceDesign
        | Product::Assistant => {
            "still (drawn now or batched), spoken line, music track, voice design, assistant call"
        }
    }
}

const ALL: [Product; 6] = [
    Product::VeoShot,
    Product::Still,
    Product::SpokenLine,
    Product::MusicTrack,
    Product::VoiceDesign,
    Product::Assistant,
];

/// `150` as the page writes it: `×1.5`.
fn times(percent: u64) -> String {
    let fraction = format!("{:02}", percent % 100);
    match fraction.trim_end_matches('0') {
        "" => format!("×{}", percent / 100),
        digits => format!("×{}.{digits}", percent / 100),
    }
}

#[test]
fn a_percentage_reads_as_the_page_writes_it() {
    assert_eq!(times(150), "×1.5");
    assert_eq!(times(180), "×1.8");
    assert_eq!(times(200), "×2");
    assert_eq!(times(105), "×1.05");
}

#[test]
fn the_multipliers_table_is_the_codes() {
    for product in ALL {
        let line = format!("| {} | {} |", row(product), times(product.percent()));
        assert!(
            PAGE.lines().any(|it| it == line),
            "docs/web.md's multipliers table should have `{line}` for {product:?}"
        );
    }
}

#[test]
fn the_summary_names_the_codes_multipliers() {
    let prose = PAGE.split_whitespace().collect::<Vec<_>>().join(" ");
    let summary = format!(
        "{} on a Veo shot, {} on the rest",
        times(Product::VeoShot.percent()),
        times(Product::Still.percent())
    );
    assert!(
        prose.contains(&summary),
        "docs/web.md's *Credits* summary should say `{summary}`"
    );
}
