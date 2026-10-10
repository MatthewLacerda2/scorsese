//! The serif faces: editorial, quotes, the documentary title.

use super::Family;

pub(super) const FAMILIES: [Family; 7] = [
    Family::drawn(
        "liberation-serif",
        "Liberation Serif",
        &[
            (400, face!("LiberationSerif-Regular.ttf")),
            (700, face!("LiberationSerif-Bold.ttf")),
        ],
        Some(&[
            (400, face!("LiberationSerif-Italic.ttf")),
            (700, face!("LiberationSerif-BoldItalic.ttf")),
        ]),
    ),
    Family::variable(
        "lora",
        "Lora",
        face!("Lora[wght].ttf"),
        Some(face!("Lora-Italic[wght].ttf")),
    ),
    Family::variable(
        "merriweather",
        "Merriweather",
        face!("Merriweather[opsz,wdth,wght].ttf"),
        Some(face!("Merriweather-Italic[opsz,wdth,wght].ttf")),
    ),
    Family::variable(
        "cormorant-garamond",
        "Cormorant Garamond",
        face!("CormorantGaramond[wght].ttf"),
        Some(face!("CormorantGaramond-Italic[wght].ttf")),
    ),
    Family::variable(
        "eb-garamond",
        "EB Garamond",
        face!("EBGaramond[wght].ttf"),
        Some(face!("EBGaramond-Italic[wght].ttf")),
    ),
    Family::variable(
        "libre-baskerville",
        "Libre Baskerville",
        face!("LibreBaskerville[wght].ttf"),
        Some(face!("LibreBaskerville-Italic[wght].ttf")),
    ),
    Family::drawn(
        "dm-serif-display",
        "DM Serif Display",
        &[(400, face!("DMSerifDisplay-Regular.ttf"))],
        Some(&[(400, face!("DMSerifDisplay-Italic.ttf"))]),
    ),
];
