//! Every colour the window uses, named once — in a light set and a dark one.
//!
//! A panel that reaches for a literal `Color32` is a panel whose look cannot be
//! changed without finding it, and three of them had drifted apart before this
//! module existed. So a panel asks [`of`] for the palette in force and reads a
//! role from it; nothing outside this file spells a colour.
//!
//! ## Where the values come from
//!
//! **The web app** (#643). Its neutrals are shadcn's tokens in
//! `web/src/index.css`, written in `oklch`, and the fields of [`Palette`] carry
//! the same names so the two can be read side by side. Each value was converted
//! to sRGB once, by the standard OKLab → linear sRGB → gamma path, and the
//! `oklch` it came from is beside it. Two of the web's dark tokens are
//! *translucent* white (`border` at 10%, `input` at 15%); egui composites in the
//! same sRGB space a browser does, so they are written here already composited
//! over `background` — opaque, and identical wherever a panel puts them.
//!
//! A handful of roles have no shadcn token, because the web never needed them
//! or draws them with Tailwind's own palette: the playhead (`red-500`, as the
//! web's timeline draws it), a warning (`amber`, the stop that reads on each
//! ground), the black matte a picture sits on (the web's `bg-black` preview),
//! and the three states of a control. Those are said where they are declared.
//!
//! ## What colour means
//!
//! The chrome is neutral — grey on grey, as the web's is — so that colour is
//! left free to mean one thing: **what a thing is**. That is why the clip hues
//! live here beside the chrome, and are the web's `kinds.ts` hue for hue: a
//! title is amber in the pool, on the timeline and in the inspector, and amber
//! in the browser too.

use egui::Color32;
use scorsese_core::{AssetKind, TrackKind};

/// One complete set of roles. There are two of them, [`DARK`] and [`LIGHT`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Palette {
    /// The window's ground and every panel's fill.
    pub(crate) background: Color32,
    /// Ordinary text.
    pub(crate) foreground: Color32,
    /// What floats: a dialog, a menu, a tooltip.
    pub(crate) card: Color32,
    /// A surface standing on a panel: a lane, a hovered row.
    pub(crate) muted: Color32,
    /// Text that is true but secondary: a heading, a unit, a hint.
    pub(crate) muted_foreground: Color32,
    /// A hairline between two things of one kind.
    pub(crate) border: Color32,
    /// A hairline with more weight: a field's outline, a panel edge.
    pub(crate) input: Color32,
    /// A focus ring, and text at the edge of legibility — a tick label, a
    /// placeholder. One value because the web gives both jobs to it.
    pub(crate) ring: Color32,
    /// The strongest neutral: what the web fills a primary button with.
    pub(crate) primary: Color32,
    /// Something that is wrong: a refused edit, a missing file.
    pub(crate) destructive: Color32,
    /// A control at rest: the fill of the web's `outline` button and its
    /// inputs, which a field and a button share here.
    pub(crate) field: Color32,
    /// The same control under the pointer.
    pub(crate) hover: Color32,
    /// A control while it is pressed or dragged, and a selected row. No web
    /// token — a browser barely draws a pressed state — so one step on from
    /// `hover` along the same neutral ramp.
    pub(crate) pressed: Color32,
    /// Something that wants attention and is not wrong. Tailwind `amber`.
    pub(crate) warning: Color32,
}

/// Dark: `.dark` in `web/src/index.css`.
pub(crate) const DARK: Palette = Palette {
    background: grey(0x0A),                           // oklch(0.145 0 0)
    foreground: grey(0xFA),                           // oklch(0.985 0 0)
    card: grey(0x17),                                 // oklch(0.205 0 0)
    muted: grey(0x26),                                // oklch(0.269 0 0)
    muted_foreground: grey(0xA1),                     // oklch(0.708 0 0)
    border: grey(0x22),                               // oklch(1 0 0 / 10%) over background
    input: grey(0x2F),                                // oklch(1 0 0 / 15%) over background
    ring: grey(0x73),                                 // oklch(0.556 0 0)
    primary: grey(0xE5),                              // oklch(0.922 0 0)
    destructive: Color32::from_rgb(0xFF, 0x64, 0x67), // oklch(0.704 0.191 22.216)
    field: grey(0x15),   // `dark:bg-input/30`: 4.5% white over background
    hover: grey(0x1C),   // `dark:hover:bg-input/50`: 7.5% white
    pressed: grey(0x26), // oklch(0.269 0 0), the web's muted
    warning: Color32::from_rgb(0xFF, 0xB9, 0x00), // amber-400, oklch(82.8% 0.189 84.429)
};

/// Light: `:root` in `web/src/index.css`.
pub(crate) const LIGHT: Palette = Palette {
    background: grey(0xFF),                           // oklch(1 0 0)
    foreground: grey(0x0A),                           // oklch(0.145 0 0)
    card: grey(0xFF),                                 // oklch(1 0 0)
    muted: grey(0xF5),                                // oklch(0.97 0 0)
    muted_foreground: grey(0x73),                     // oklch(0.556 0 0)
    border: grey(0xE5),                               // oklch(0.922 0 0)
    input: grey(0xE5),                                // oklch(0.922 0 0)
    ring: grey(0xA1),                                 // oklch(0.708 0 0)
    primary: grey(0x17),                              // oklch(0.205 0 0)
    destructive: Color32::from_rgb(0xE7, 0x00, 0x0B), // oklch(0.577 0.245 27.325)
    field: grey(0xFF),                                // `bg-background`
    hover: grey(0xF5),                                // `hover:bg-muted`, oklch(0.97 0 0)
    pressed: grey(0xE5),                              // oklch(0.922 0 0), the web's border
    warning: Color32::from_rgb(0xE1, 0x71, 0x00),     // amber-600, oklch(66.6% 0.179 58.318)
};

/// The playhead, in both themes: Tailwind `red-500`, `oklch(63.7% 0.237
/// 25.331)`, the web timeline's `bg-red-500`. Its own colour so it is never
/// mistaken for a clip, a rule or a refusal.
pub(crate) const PLAYHEAD: Color32 = Color32::from_rgb(0xFB, 0x2C, 0x36);

/// What a picture sits on, in both themes: black, as the web's preview is.
///
/// Not the panel's ground. A picture is judged against its surround, and a white
/// surround would make every shot look darker than it will play — so the light
/// theme lightens the window and never the matte.
pub(crate) const MATTE: Color32 = Color32::BLACK;

/// The palette in force in `ctx`: whichever theme egui has resolved, which is
/// the person's explicit choice or else the system's (see `super::choice`).
pub(crate) fn of(ctx: &egui::Context) -> &'static Palette {
    match ctx.theme() {
        egui::Theme::Dark => &DARK,
        egui::Theme::Light => &LIGHT,
    }
}

/// A neutral, since every web token but three is one.
const fn grey(level: u8) -> Color32 {
    Color32::from_rgb(level, level, level)
}

/// The hue that says what an asset is: `web/src/editor/assets/kinds.ts`.
///
/// The web spells these as Tailwind classes; the values are Tailwind v4's
/// (`tailwindcss` 4.3, the version `web/package.json` pins), converted from
/// their `oklch` like the neutrals. Families follow the web: picture is sky,
/// stills teal, sound green, words amber, drawings purple — and a generated
/// asset is the lighter or neighbouring stop of its family, so an unmade shot
/// still reads as a shot. The same in both themes, as on the web.
pub(crate) const fn of_kind(kind: AssetKind) -> Color32 {
    match kind {
        AssetKind::Video => Color32::from_rgb(0x00, 0xA6, 0xF4), // sky-500
        AssetKind::GeneratedVideo => Color32::from_rgb(0x00, 0xBC, 0xFF), // sky-400
        AssetKind::Image => Color32::from_rgb(0x00, 0xBB, 0xA7), // teal-500
        AssetKind::GeneratedImage => Color32::from_rgb(0x00, 0xD5, 0xBE), // teal-400
        AssetKind::ImageSequence => Color32::from_rgb(0x00, 0x96, 0x89), // teal-600
        AssetKind::Audio => Color32::from_rgb(0x00, 0xBC, 0x7D), // emerald-500
        AssetKind::GeneratedAudio => Color32::from_rgb(0x7C, 0xCF, 0x00), // lime-500
        AssetKind::SynthAudio => Color32::from_rgb(0x00, 0xA6, 0x3E), // green-600
        AssetKind::Text => Color32::from_rgb(0xFE, 0x9A, 0x00),  // amber-500
        AssetKind::Color => Color32::from_rgb(0xE1, 0x2A, 0xFB), // fuchsia-500
        AssetKind::Shape => Color32::from_rgb(0x8E, 0x51, 0xFF), // violet-500
        AssetKind::Icon => Color32::from_rgb(0x61, 0x5F, 0xFF),  // indigo-500
        AssetKind::Group => Color32::from_rgb(0xAD, 0x46, 0xFF), // purple-500
    }
}

/// The hue that says what a *lane* carries: its commonest member's, so a track
/// head and the blocks along it agree.
pub(crate) const fn of_track(kind: TrackKind) -> Color32 {
    match kind {
        TrackKind::Video => of_kind(AssetKind::Video),
        TrackKind::Audio => of_kind(AssetKind::Audio),
    }
}

/// A clip whose asset is not in the table: the web's grey for a kind it does
/// not know, `zinc-500`, `oklch(55.2% 0.016 285.938)`. A validated project
/// never reaches it.
pub(crate) const UNKNOWN: Color32 = Color32::from_rgb(0x71, 0x71, 0x7B);

/// A hue mixed towards a ground: `amount` 1.0 is the hue, 0.0 the ground.
///
/// Blended here rather than with `gamma_multiply`, which darkens towards black
/// and takes the colour out of a hue along with the light — on a light ground
/// that would be the wrong direction entirely.
pub(crate) fn over(hue: Color32, ground: Color32, amount: f32) -> Color32 {
    let mix = |a: u8, b: u8| (f32::from(b) + (f32::from(a) - f32::from(b)) * amount) as u8;
    Color32::from_rgb(
        mix(hue.r(), ground.r()),
        mix(hue.g(), ground.g()),
        mix(hue.b(), ground.b()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINDS: [AssetKind; 13] = [
        AssetKind::Video,
        AssetKind::Image,
        AssetKind::Audio,
        AssetKind::Text,
        AssetKind::Color,
        AssetKind::Shape,
        AssetKind::Icon,
        AssetKind::Group,
        AssetKind::ImageSequence,
        AssetKind::GeneratedVideo,
        AssetKind::GeneratedImage,
        AssetKind::GeneratedAudio,
        AssetKind::SynthAudio,
    ];

    /// The ends of the mix are the two colours themselves.
    #[test]
    fn a_mix_of_all_or_nothing_is_one_colour_or_the_other() {
        let hue = of_kind(AssetKind::Text);
        assert_eq!(over(hue, DARK.background, 1.0), hue);
        assert_eq!(over(hue, DARK.background, 0.0), DARK.background);
    }

    /// Each theme is the other turned over: dark text on a light ground and
    /// light on dark, with the secondary text between them. A palette that
    /// swapped one role and not its partner would be unreadable in one theme.
    #[test]
    fn each_theme_keeps_its_text_off_its_ground() {
        for palette in [DARK, LIGHT] {
            let (ground, text, quiet) = (
                palette.background.r(),
                palette.foreground.r(),
                palette.muted_foreground.r(),
            );
            assert!(ground.abs_diff(text) > 200, "{palette:?}");
            assert!(ground.abs_diff(quiet) > 90, "{palette:?}");
        }
        assert!(DARK.background.r() < LIGHT.background.r());
    }

    /// Every kind has its own hue — the web gives each one its own — and none
    /// is a ground of either theme, where a clip would vanish.
    #[test]
    fn every_kind_is_its_own_and_visible_in_both_themes() {
        for (index, kind) in KINDS.into_iter().enumerate() {
            for palette in [DARK, LIGHT] {
                assert_ne!(of_kind(kind), palette.muted, "{kind:?}");
                assert_ne!(of_kind(kind), palette.background, "{kind:?}");
            }
            for other in &KINDS[index + 1..] {
                assert_ne!(of_kind(kind), of_kind(*other), "{kind:?} and {other:?}");
            }
        }
    }

    /// The palette follows the theme egui has resolved, which is how a choice
    /// in the bar or a change in the system reaches every panel at once.
    #[test]
    fn the_palette_in_force_is_the_theme_in_force() {
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Light);
        assert_eq!(*of(&ctx), LIGHT);
        ctx.set_theme(egui::Theme::Dark);
        assert_eq!(*of(&ctx), DARK);
    }
}
