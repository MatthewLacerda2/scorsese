//! The rounded, friendly faces: children's videos, church, the classroom.

use super::Family;

pub(super) const FAMILIES: [Family; 5] = [
    Family::variable(
        "nunito",
        "Nunito",
        face!("Nunito[wght].ttf"),
        Some(face!("Nunito-Italic[wght].ttf")),
    ),
    Family::variable("quicksand", "Quicksand", face!("Quicksand[wght].ttf"), None),
    Family::variable("fredoka", "Fredoka", face!("Fredoka[wdth,wght].ttf"), None),
    Family::variable("baloo-2", "Baloo 2", face!("Baloo2[wght].ttf"), None),
    Family::variable("comfortaa", "Comfortaa", face!("Comfortaa[wght].ttf"), None),
];
