//! The rest of [`super::variants`]' table: the enums whose variants carry
//! nothing, which [`every!`] lists by name alone.

use scorsese_core::{
    ImageAspect, ImageResolution, ImageThinking, Locale, OriginX, OriginY, RevealUnit, Side,
    SpeechModel, StockLibrary, TextAlign, TrackKind, VideoModel, VideoResolution,
};
use serde_json::Value;

pub(crate) fn more(put: &mut impl FnMut(&str, Value) -> Option<Value>) {
    put("StockLibrary", every!(StockLibrary::Pixabay));
    put(
        "ImageAspect",
        every!(
            ImageAspect::Wide,
            ImageAspect::Tall,
            ImageAspect::Square,
            ImageAspect::FourThree,
            ImageAspect::ThreeFour,
            ImageAspect::ThreeTwo,
            ImageAspect::TwoThree,
            ImageAspect::FiveFour,
            ImageAspect::FourFive,
            ImageAspect::Cinema,
            ImageAspect::FourOne,
            ImageAspect::OneFour,
            ImageAspect::EightOne,
            ImageAspect::OneEight,
        ),
    );
    put(
        "ImageResolution",
        every!(
            ImageResolution::K1,
            ImageResolution::K2,
            ImageResolution::K4
        ),
    );
    put(
        "ImageThinking",
        every!(
            ImageThinking::Minimal,
            ImageThinking::Medium,
            ImageThinking::High
        ),
    );
    put("Locale", every!(Locale::En, Locale::PtBr));
    put(
        "OriginX",
        every!(OriginX::Left, OriginX::Center, OriginX::Right),
    );
    put(
        "OriginY",
        every!(OriginY::Top, OriginY::Center, OriginY::Bottom),
    );
    put(
        "RevealUnit",
        every!(RevealUnit::Char, RevealUnit::Word, RevealUnit::Line),
    );
    put(
        "Side",
        every!(
            Side::Left,
            Side::Right,
            Side::Top,
            Side::Bottom,
            Side::Center
        ),
    );
    put(
        "SpeechModel",
        every!(
            SpeechModel::Expressive,
            SpeechModel::Standard,
            SpeechModel::Fast
        ),
    );
    put(
        "TextAlign",
        every!(TextAlign::Left, TextAlign::Center, TextAlign::Right),
    );
    put("TrackKind", every!(TrackKind::Video, TrackKind::Audio));
    put(
        "VideoModel",
        every!(VideoModel::Standard, VideoModel::Fast, VideoModel::Lite),
    );
    put(
        "VideoResolution",
        every!(
            VideoResolution::P720,
            VideoResolution::P1080,
            VideoResolution::P4k,
        ),
    );
}
