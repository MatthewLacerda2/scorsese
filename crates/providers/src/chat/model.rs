//! The models the assistant offers, and whose they are.

use std::time::Duration;

use crate::credentials::Provider;

/// One model the hosted assistant can run on (#705). Exactly these two: what
/// a user picks between in the chat panel, and what [`crate::prices::chat`]
/// has a row for. Gemini was offered too until the maintainer cut it on
/// 2026-10-12 as not good enough at the job (#1220).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Model {
    /// Claude Opus 5.5 — the strongest, and the dearest.
    ClaudeOpus55,
    /// Claude Sonnet 5.5 — the default (the maintainer, 2026-10-07: the
    /// balance of quality and price).
    ClaudeSonnet55,
}

/// Who serves a model: which wire its requests speak, and whose key pays.
///
/// One vendor today, kept an enum anyway (#1220) so that a second is an
/// addition to the seam rather than a redesign of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vendor {
    /// Anthropic's Messages API.
    Anthropic,
}

impl Model {
    /// Every model offered, in the order the picker lists them.
    pub const ALL: [Self; 2] = [Self::ClaudeSonnet55, Self::ClaudeOpus55];

    /// What a project runs on until its owner picks otherwise.
    pub const DEFAULT: Self = Self::ClaudeSonnet55;

    /// The id the vendor's API answers to, and what the server stores.
    pub const fn id(self) -> &'static str {
        match self {
            Self::ClaudeOpus55 => "claude-opus-5-5",
            Self::ClaudeSonnet55 => "claude-sonnet-5-5",
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
        }
    }

    /// How long what this model's conversations leave in a cache outlives the
    /// last call — what the web app's switch warning waits out (#705).
    ///
    /// An hour on Claude: the longest entry the request writes
    /// ([`crate::claude`], the tools and system prompt at `1h`).
    pub const fn cache_lifetime(self) -> Duration {
        match self.vendor() {
            Vendor::Anthropic => Duration::from_secs(60 * 60),
        }
    }

    /// Who serves it.
    pub const fn vendor(self) -> Vendor {
        match self {
            Self::ClaudeOpus55 | Self::ClaudeSonnet55 => Vendor::Anthropic,
        }
    }
}

impl Vendor {
    /// Its name, as a record of model use stores it: `anthropic` (older
    /// records also hold `google`, from when Gemini was offered).
    pub const fn id(self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
        }
    }

    /// The key it is reached with, from the one credentials resolver.
    pub const fn provider(self) -> Provider {
        match self {
            Self::Anthropic => Provider::Anthropic,
        }
    }
}
