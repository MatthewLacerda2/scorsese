//! Handwriting, marker and script: the whiteboard, the quote, the wedding
//! invitation. Drawn by hand, so drawn upright and almost always at one weight.

use super::Family;

pub(super) const FAMILIES: [Family; 9] = [
    Family::variable("caveat", "Caveat", face!("Caveat[wght].ttf"), None),
    Family::drawn(
        "patrick-hand",
        "Patrick Hand",
        &[(400, face!("PatrickHand-Regular.ttf"))],
        None,
    ),
    Family::drawn(
        "kalam",
        "Kalam",
        &[
            (300, face!("Kalam-Light.ttf")),
            (400, face!("Kalam-Regular.ttf")),
            (700, face!("Kalam-Bold.ttf")),
        ],
        None,
    ),
    Family::drawn(
        "gochi-hand",
        "Gochi Hand",
        &[(400, face!("GochiHand-Regular.ttf"))],
        None,
    ),
    Family::drawn(
        "indie-flower",
        "Indie Flower",
        &[(400, face!("IndieFlower-Regular.ttf"))],
        None,
    ),
    Family::drawn(
        "permanent-marker",
        "Permanent Marker",
        &[(400, face!("PermanentMarker-Regular.ttf"))],
        None,
    ),
    Family::variable(
        "dancing-script",
        "Dancing Script",
        face!("DancingScript[wght].ttf"),
        None,
    ),
    Family::drawn(
        "pacifico",
        "Pacifico",
        &[(400, face!("Pacifico-Regular.ttf"))],
        None,
    ),
    Family::drawn(
        "great-vibes",
        "Great Vibes",
        &[(400, face!("GreatVibes-Regular.ttf"))],
        None,
    ),
];
