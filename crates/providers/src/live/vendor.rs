//! Who the check calls: a vendor reached with a key, or one that needs none.
//!
//! The check began keyed by [`Provider`], because every vendor it called
//! needed a key and a skip was always *no key*. LottieFiles (#910) answers its
//! public search anonymously, so it has no [`Provider`] and never will — a
//! credential nobody needs is not something to add to the resolver. Rather than
//! a second list run beside the first, a vendor is one of two kinds, and the
//! plan, the run and the report go through the same steps for both: a keyed
//! vendor's key comes out of the resolver, a keyless one has none to find.

use crate::credentials::Provider;

/// One vendor the check calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vendor {
    /// A vendor reached with a key from the one resolver.
    Keyed(Provider),
    /// LottieFiles' public GraphQL, answered anonymously (#903).
    LottieFiles,
}

impl Vendor {
    /// The key this vendor is reached with, if it needs one.
    pub const fn provider(self) -> Option<Provider> {
        match self {
            Self::Keyed(provider) => Some(provider),
            Self::LottieFiles => None,
        }
    }

    /// The vendor as a report names it.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Keyed(provider) => provider.label(),
            Self::LottieFiles => "LottieFiles (free animations, no key)",
        }
    }

    /// The vendor as a recorded file's name starts: `gemini`, `elevenlabs`,
    /// `anthropic`, `pixabay`, `lottiefiles`.
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Keyed(Provider::Gemini) => "gemini",
            Self::Keyed(Provider::ElevenLabs) => "elevenlabs",
            Self::Keyed(Provider::Anthropic) => "anthropic",
            Self::Keyed(Provider::Pixabay) => "pixabay",
            Self::LottieFiles => "lottiefiles",
        }
    }
}
