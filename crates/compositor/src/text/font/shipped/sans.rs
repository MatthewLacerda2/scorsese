//! The everyday sans faces: captions, interfaces, the corporate explainer.

use super::Family;

pub(super) const FAMILIES: [Family; 10] = [
    Family::drawn(
        "liberation-sans",
        "Liberation Sans",
        &[
            (400, face!("LiberationSans-Regular.ttf")),
            (700, face!("LiberationSans-Bold.ttf")),
        ],
        Some(&[
            (400, face!("LiberationSans-Italic.ttf")),
            (700, face!("LiberationSans-BoldItalic.ttf")),
        ]),
    ),
    Family::variable(
        "montserrat",
        "Montserrat",
        face!("Montserrat[wght].ttf"),
        Some(face!("Montserrat-Italic[wght].ttf")),
    ),
    // Poppins and Lato have no variable build: every weight is its own file,
    // keyed by the CSS weight Google Fonts serves it as (Thin is 100 here even
    // where the file's own `usWeightClass` says 250).
    Family::drawn(
        "poppins",
        "Poppins",
        &[
            (100, face!("Poppins-Thin.ttf")),
            (200, face!("Poppins-ExtraLight.ttf")),
            (300, face!("Poppins-Light.ttf")),
            (400, face!("Poppins-Regular.ttf")),
            (500, face!("Poppins-Medium.ttf")),
            (600, face!("Poppins-SemiBold.ttf")),
            (700, face!("Poppins-Bold.ttf")),
            (800, face!("Poppins-ExtraBold.ttf")),
            (900, face!("Poppins-Black.ttf")),
        ],
        Some(&[
            (100, face!("Poppins-ThinItalic.ttf")),
            (200, face!("Poppins-ExtraLightItalic.ttf")),
            (300, face!("Poppins-LightItalic.ttf")),
            (400, face!("Poppins-Italic.ttf")),
            (500, face!("Poppins-MediumItalic.ttf")),
            (600, face!("Poppins-SemiBoldItalic.ttf")),
            (700, face!("Poppins-BoldItalic.ttf")),
            (800, face!("Poppins-ExtraBoldItalic.ttf")),
            (900, face!("Poppins-BlackItalic.ttf")),
        ]),
    ),
    Family::variable(
        "roboto",
        "Roboto",
        face!("Roboto[wdth,wght].ttf"),
        Some(face!("Roboto-Italic[wdth,wght].ttf")),
    ),
    Family::variable(
        "open-sans",
        "Open Sans",
        face!("OpenSans[wdth,wght].ttf"),
        Some(face!("OpenSans-Italic[wdth,wght].ttf")),
    ),
    Family::drawn(
        "lato",
        "Lato",
        &[
            (100, face!("Lato-Thin.ttf")),
            (200, face!("Lato-ExtraLight.ttf")),
            (300, face!("Lato-Light.ttf")),
            (400, face!("Lato-Regular.ttf")),
            (500, face!("Lato-Medium.ttf")),
            (600, face!("Lato-SemiBold.ttf")),
            (700, face!("Lato-Bold.ttf")),
            (800, face!("Lato-ExtraBold.ttf")),
            (900, face!("Lato-Black.ttf")),
        ],
        Some(&[
            (100, face!("Lato-ThinItalic.ttf")),
            (200, face!("Lato-ExtraLightItalic.ttf")),
            (300, face!("Lato-LightItalic.ttf")),
            (400, face!("Lato-Italic.ttf")),
            (500, face!("Lato-MediumItalic.ttf")),
            (600, face!("Lato-SemiBoldItalic.ttf")),
            (700, face!("Lato-BoldItalic.ttf")),
            (800, face!("Lato-ExtraBoldItalic.ttf")),
            (900, face!("Lato-BlackItalic.ttf")),
        ]),
    ),
    Family::variable(
        "raleway",
        "Raleway",
        face!("Raleway[wght].ttf"),
        Some(face!("Raleway-Italic[wght].ttf")),
    ),
    Family::variable(
        "dm-sans",
        "DM Sans",
        face!("DMSans[opsz,wght].ttf"),
        Some(face!("DMSans-Italic[opsz,wght].ttf")),
    ),
    Family::variable(
        "work-sans",
        "Work Sans",
        face!("WorkSans[wght].ttf"),
        Some(face!("WorkSans-Italic[wght].ttf")),
    ),
    Family::variable(
        "rubik",
        "Rubik",
        face!("Rubik[wght].ttf"),
        Some(face!("Rubik-Italic[wght].ttf")),
    ),
];
