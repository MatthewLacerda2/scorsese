//! What a still can be asked for: Nano Banana 2.1's limits.
//!
//! Every still is drawn by **Nano Banana 2.1** (`gemini-nano-banana-2.1`), and
//! by nothing else (#1308): one model is not a choice, so nothing in a brief
//! names it. What used to be asked of a model is answered here, once.
//!
//! **Every limit here is Google's, as documented**, read off
//! <https://ai.google.dev/gemini-api/docs/image-generation> (last updated
//! 2026-10-09 UTC) on 2026-10-15 — the parity rule, #891: scorsese offers what
//! the provider's API offers, so the page is the checklist. It draws every one
//! of the fourteen [`ImageAspect`](super::ImageAspect)s at 1K, 2K and 4K;
//! takes `minimal`, `medium` and `high` thinking; and keeps up to ten objects
//! and four characters from their reference pictures.

use super::{ImageResolution, ImageThinking, ReferenceKind};

/// The level a still is drawn at when its request names none.
pub const DEFAULT_IMAGE_THINKING: ImageThinking = ImageThinking::Medium;

/// The size a still is drawn at when its request names none.
///
/// 2K: a 16:9 2K still is 2752×1536, which covers a 1080p frame with room left
/// for the slow push or the pan a still is usually given — at 1K (1376×768) it
/// would be enlarged to fill the frame before anything moved.
pub const DEFAULT_IMAGE_RESOLUTION: ImageResolution = ImageResolution::K2;

impl ReferenceKind {
    /// The most references of this kind one still may name: ten objects and
    /// four characters (Google's *Limitations*).
    pub const fn max(self) -> usize {
        match self {
            Self::Object => 10,
            Self::Character => 4,
        }
    }
}
