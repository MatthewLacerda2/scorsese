//! How large and what shape a still comes back: two independent choices.

use serde::{Deserialize, Serialize};

/// How large a generated still comes back, named the way the vendor prices it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageResolution {
    /// 512 pixels on the long side, roughly. The cheapest, for a thumbnail or
    /// a test of a prompt. Flash only.
    #[serde(rename = "0.5K")]
    K05,
    /// About 1024 on the long side. The only size Lite draws.
    #[serde(rename = "1K")]
    K1,
    /// About 2048 on the long side — the default wherever it is drawn.
    #[serde(rename = "2K")]
    K2,
    /// About 4096 on the long side. For a still that is pushed into a long way.
    #[serde(rename = "4K")]
    K4,
}

impl ImageResolution {
    /// Every size, smallest first.
    pub const ALL: [Self; 4] = [Self::K05, Self::K1, Self::K2, Self::K4];

    /// How it is written in `project.json` and in a price table.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::K05 => "0.5K",
            Self::K1 => "1K",
            Self::K2 => "2K",
            Self::K4 => "4K",
        }
    }
}

/// Which shape a generated still is.
///
/// Every ratio any model draws; [`ImageModel::draws`](super::ImageModel::draws)
/// says which model draws which. Baked into the file the vendor returns, so it
/// is a fact about the asset — and a still that is not the render's shape is
/// not wrong, only fitted, the way any imported photograph is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ImageAspect {
    /// Landscape, and the default — the shape most cuts are.
    #[default]
    #[serde(rename = "16:9")]
    Wide,
    /// Portrait, for a cut watched upright.
    #[serde(rename = "9:16")]
    Tall,
    /// Square.
    #[serde(rename = "1:1")]
    Square,
    /// The older television shape, landscape.
    #[serde(rename = "4:3")]
    FourThree,
    /// The older television shape, portrait.
    #[serde(rename = "3:4")]
    ThreeFour,
    /// A photograph, landscape.
    #[serde(rename = "3:2")]
    ThreeTwo,
    /// A photograph, portrait.
    #[serde(rename = "2:3")]
    TwoThree,
    /// A print, landscape.
    #[serde(rename = "5:4")]
    FiveFour,
    /// A print, portrait — the shape of most social feeds.
    #[serde(rename = "4:5")]
    FourFive,
    /// Cinema-wide, for a backdrop panned across.
    #[serde(rename = "21:9")]
    Cinema,
    /// A banner, four wide to one high: a panorama to pan along. 2.1 and
    /// Flash only.
    #[serde(rename = "4:1")]
    FourOne,
    /// A column, one wide to four high: a long scroll down. 2.1 and Flash only.
    #[serde(rename = "1:4")]
    OneFour,
    /// A long banner, eight to one. 2.1 and Flash only.
    #[serde(rename = "8:1")]
    EightOne,
    /// A long column, one to eight. 2.1 and Flash only.
    #[serde(rename = "1:8")]
    OneEight,
}

impl ImageAspect {
    /// Every ratio, in the order a picker should offer them.
    pub const ALL: [Self; 14] = [
        Self::Wide,
        Self::Tall,
        Self::Square,
        Self::FourThree,
        Self::ThreeFour,
        Self::ThreeTwo,
        Self::TwoThree,
        Self::FiveFour,
        Self::FourFive,
        Self::Cinema,
        Self::FourOne,
        Self::OneFour,
        Self::EightOne,
        Self::OneEight,
    ];

    /// The ratio as `project.json` and the vendor both spell it.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Wide => "16:9",
            Self::Tall => "9:16",
            Self::Square => "1:1",
            Self::FourThree => "4:3",
            Self::ThreeFour => "3:4",
            Self::ThreeTwo => "3:2",
            Self::TwoThree => "2:3",
            Self::FiveFour => "5:4",
            Self::FourFive => "4:5",
            Self::Cinema => "21:9",
            Self::FourOne => "4:1",
            Self::OneFour => "1:4",
            Self::EightOne => "8:1",
            Self::OneEight => "1:8",
        }
    }

    /// True for the four long strips only some models draw.
    pub const fn is_strip(self) -> bool {
        matches!(
            self,
            Self::FourOne | Self::OneFour | Self::EightOne | Self::OneEight
        )
    }
}
