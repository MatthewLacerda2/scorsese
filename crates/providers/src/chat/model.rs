//! The models the assistant offers, and whose they are.

use std::time::Duration;

use crate::credentials::Provider;

/// One model the hosted assistant can run on (#705). Exactly these four: what
/// a user picks between in the chat panel, and what [`crate::prices::chat`]
/// has a row for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    /// Claude Opus 5.5 — the strongest, and the dearest.
    ClaudeOpus55,
    /// Claude Sonnet 5.5 — the default (the maintainer, 2026-10-07: the
    /// balance of quality and price).
    ClaudeSonnet55,
    /// Gemini 3.8 Flash.
    GeminiFlash38,
    /// Gemini 3.5 Flash Lite — the cheapest. There is no 3.8 Lite.
    GeminiFlashLite35,
}

/// Who serves a model: which wire its requests speak, and whose key pays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vendor {
    /// Anthropic's Messages API.
    Anthropic,
    /// Google's Gemini API (`generateContent`).
    Google,
}

impl Model {
    /// Every model offered, in the order the picker lists them.
    pub const ALL: [Self; 4] = [
        Self::GeminiFlash38,
        Self::GeminiFlashLite35,
        Self::ClaudeOpus55,
        Self::ClaudeSonnet55,
    ];

    /// What a project runs on until its owner picks otherwise.
    pub const DEFAULT: Self = Self::ClaudeSonnet55;

    /// The id the vendor's API answers to, and what the server stores.
    pub const fn id(self) -> &'static str {
        match self {
            Self::ClaudeOpus55 => "claude-opus-5-5",
            Self::ClaudeSonnet55 => "claude-sonnet-5-5",
            Self::GeminiFlash38 => "gemini-3.8-flash",
            Self::GeminiFlashLite35 => "gemini-3.5-flash-lite",
        }
    }

    /// The model by its id, if it is one offered.
    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|model| model.id() == id)
    }

    /// Its name, as a person reads it.
    pub const fn label(self) -> &'static str {
        match self {
            Self::ClaudeOpus55 => "Claude Opus 5.5",
            Self::ClaudeSonnet55 => "Claude Sonnet 5.5",
            Self::GeminiFlash38 => "Gemini 3.8 Flash",
            Self::GeminiFlashLite35 => "Gemini 3.5 Flash Lite",
        }
    }

    /// How long what this model's conversations leave in a cache outlives the
    /// last call — what the web app's switch warning waits out (#705).
    ///
    /// An hour on Claude: the longest entry the request writes
    /// ([`crate::claude`], the tools and system prompt at `1h`). An hour on
    /// Gemini too, though for a different reason: Google documents no lifetime
    /// for its implicit cache, so the hour is the cautious answer — a warning
    /// shown when the cache has already gone costs a click, one withheld while
    /// it is still warm costs the user money.
    pub const fn cache_lifetime(self) -> Duration {
        match self.vendor() {
            Vendor::Anthropic | Vendor::Google => Duration::from_secs(60 * 60),
        }
    }

    /// Who serves it.
    pub const fn vendor(self) -> Vendor {
        match self {
            Self::ClaudeOpus55 | Self::ClaudeSonnet55 => Vendor::Anthropic,
            Self::GeminiFlash38 | Self::GeminiFlashLite35 => Vendor::Google,
        }
    }
}

impl Vendor {
    /// Its name, as a record of model use stores it: `anthropic` or `google`.
    pub const fn id(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::Google => "google",
        }
    }

    /// The key it is reached with, from the one credentials resolver. Google's
    /// is the key Veo and the image models already spend.
    pub const fn provider(self) -> Provider {
        match self {
            Self::Anthropic => Provider::Anthropic,
            Self::Google => Provider::Gemini,
        }
    }
}
