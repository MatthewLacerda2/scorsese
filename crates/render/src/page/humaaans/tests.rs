//! Every part of the pack, as a page is served it.

use super::*;

fn served(file: &str) -> String {
    String::from_utf8(serve(file).unwrap_or_else(|note| panic!("{note}"))).expect("SVG is text")
}

/// How many of each drawn mark a document has.
fn marks(svg: &str) -> Vec<usize> {
    [
        "<path", "<polygon", "<rect", "<circle", "<ellipse", "<use", "<mask",
    ]
    .iter()
    .map(|mark| svg.matches(mark).count())
    .collect()
}

#[test]
fn every_part_is_served_whole_and_without_what_sketch_left_in() {
    for part in PARTS {
        let svg = served(&format!("{}/{}.svg", part.kind, part.name));
        assert!(
            svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\""),
            "{}",
            part.name
        );
        assert_eq!(
            marks(&svg),
            marks(part.source),
            "{}/{} lost a mark",
            part.kind,
            part.name
        );
        for leftover in [
            "<title",
            "<desc",
            "<!--",
            "xlink",
            "stroke=\"none\"",
            "id=\"Head",
        ] {
            assert!(
                !svg.contains(leftover),
                "{}/{}: {leftover}",
                part.kind,
                part.name
            );
        }
    }
}

#[test]
fn an_untouched_part_keeps_the_artists_colours() {
    // Every settable colour falls back to the fill it replaces.
    for part in PARTS {
        let svg = served(&format!("{}/{}.svg", part.kind, part.name));
        for (at, _) in svg.match_indices(" style=\"fill:var(--person-") {
            let tag = &svg[svg[..at].rfind('<').expect("in a tag")..at];
            let fill = &tag[tag.find(" fill=\"").expect("a fill") + 7..][..7];
            let fallback = &svg[at..][svg[at..].find(',').expect("a fallback") + 1..][..7];
            assert_eq!(fill, fallback, "{}/{}", part.kind, part.name);
        }
    }
}

#[test]
fn each_region_is_the_colour_it_names() {
    let afro = served("head/afro.svg");
    assert!(afro.contains("fill:var(--person-hair,#191847)"));
    assert!(afro.contains("fill:var(--person-skin,#B28B67)"));
    let hoodie = served("body/hoodie.svg");
    assert!(hoodie.contains("fill:var(--person-top,#FF9B21)"));
    assert!(hoodie.contains("fill:var(--person-top-shade,#E87613)"));
    assert!(hoodie.contains("fill:var(--person-shirt,#DDE3E9)"));
    // A top with no coat over it is all top.
    assert!(served("body/pregnant.svg").contains("fill:var(--person-top,#C1DEE2)"));
    let skirt = served("standing/skirt.svg");
    assert!(skirt.contains("fill:var(--person-bottom,#2B44FF)"));
    assert!(skirt.contains("fill:var(--person-bottom-shade,#1F28CF)"));
    assert!(skirt.contains("fill:var(--person-shoes,#E4E4E4)"));
    // Two legs under one name: the darker is the far one.
    let jeans = served("standing/skinny-jeans.svg");
    assert!(jeans.contains("fill:var(--person-bottom-shade,#191847)"));
    assert!(jeans.contains("fill:var(--person-bottom,#2F3676)"));
    // The same colour is the near leg of one pair and the far leg of another.
    assert!(served("standing/sprint.svg").contains("fill:var(--person-bottom-shade,#2F3676)"));
    assert!(served("standing/jogging.svg").contains("fill:var(--person-bottom-shade,#DB2721)"));
    assert!(served("body/pointing-forward.svg").contains("fill:var(--person-top-shade,#2026A2)"));
    assert!(served("sitting/sweat-pants.svg").contains("fill:var(--person-seat,#C5CFD6)"));
    assert!(served("sitting/skinny-jeans.svg").contains("fill:var(--person-seat,#C5CFD6)"));
    // The shading laid over colours, and a wheelchair, are not clothing.
    assert!(!served("body/jacket.svg").contains("--person-top,#000000"));
    assert!(!served("sitting/wheelchair.svg").contains("#2F3676)"));
    // A whole person is recoloured the same way as its parts.
    assert!(served("person/standing-1.svg").contains("fill:var(--person-hair,"));
}

#[test]
fn every_region_is_used_and_listed() {
    let all: String = PARTS
        .iter()
        .map(|part| served(&format!("{}/{}.svg", part.kind, part.name)))
        .collect();
    for region in REGIONS {
        assert!(
            all.contains(&format!("--person-{region},")),
            "{region} is never used"
        );
    }
    for (at, _) in all.match_indices("--person-") {
        let region = all[at + 9..].split(',').next().expect("a name");
        assert!(REGIONS.contains(&region), "{region} is not listed");
    }
}

#[test]
fn parts_of_a_person_share_one_frame_and_props_keep_their_own() {
    let head = served("head/afro.svg");
    assert!(head.contains("viewBox=\"0 0 300 426\"") && head.contains("translate(82 0)"));
    assert!(served("body/jacket.svg").contains("translate(22 82)"));
    assert!(served("sitting/wheelchair.svg").contains("translate(0 187)"));
    assert!(served("scene/home.svg").contains("viewBox=\"0 0 740 680\""));
    assert!(served("person/sitting-1.svg").contains("viewBox=\"0 0 380 400\""));
}

#[test]
fn a_mask_is_named_after_its_part() {
    let turban = served("head/turban-1.svg");
    assert!(turban.contains("id=\"humaaans-head-turban-1-mask-2\""));
    assert!(turban.contains("mask=\"url(#humaaans-head-turban-1-mask-2)\""));
    assert!(turban.contains("href=\"#humaaans-head-turban-1-path-1\""));
}

#[test]
fn the_index_lists_what_is_served() {
    let index: serde_json::Value = serde_json::from_slice(&serve("index.json").unwrap()).unwrap();
    assert_eq!(index["frame"]["height"], 426);
    let listed: usize = [
        "head", "body", "standing", "sitting", "seat", "scene", "person",
    ]
    .iter()
    .map(|kind| index["parts"][kind].as_array().map_or(0, Vec::len))
    .sum();
    assert_eq!(listed, PARTS.len());
    assert!(index["licence"].as_str().unwrap().contains("CC0"));
}

#[test]
fn a_wrong_kind_names_the_kinds() {
    let note = serve("hat/fedora.svg").unwrap_err();
    assert!(
        note.contains("`hat/fedora`") && note.contains("head, body, standing"),
        "{note}"
    );
}
