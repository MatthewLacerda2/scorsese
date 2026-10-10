//! Display faces — thumbnails, hooks, memes, sport, the title card — and the
//! one monospace, which is display of a different kind.

use super::Family;

pub(super) const FAMILIES: [Family; 8] = [
    Family::variable(
        "playfair-display",
        "Playfair Display",
        face!("PlayfairDisplay[wght].ttf"),
        Some(face!("PlayfairDisplay-Italic[wght].ttf")),
    ),
    Family::drawn("anton", "Anton", &[(400, face!("Anton-Regular.ttf"))], None),
    Family::drawn(
        "bebas-neue",
        "Bebas Neue",
        &[(400, face!("BebasNeue-Regular.ttf"))],
        None,
    ),
    Family::variable("oswald", "Oswald", face!("Oswald[wght].ttf"), None),
    Family::drawn(
        "archivo-black",
        "Archivo Black",
        &[(400, face!("ArchivoBlack-Regular.ttf"))],
        None,
    ),
    Family::drawn(
        "bangers",
        "Bangers",
        &[(400, face!("Bangers-Regular.ttf"))],
        None,
    ),
    Family::drawn(
        "lilita-one",
        "Lilita One",
        &[(400, face!("LilitaOne-Regular.ttf"))],
        None,
    ),
    Family::variable(
        "jetbrains-mono",
        "JetBrains Mono",
        face!("JetBrainsMono[wght].ttf"),
        Some(face!("JetBrainsMono-Italic[wght].ttf")),
    ),
];
