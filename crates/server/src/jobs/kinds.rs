//! The kinds of job the server runs, how many of each at once, and the
//! registry their handlers are plugged into.
//!
//! The limits are for the machine this runs on: four cores and 16 GB, shared
//! with Postgres and the other containers. What uses the CPU is kept to a few
//! at a time; what waits on a provider's servers barely uses the machine and
//! is limited only so one person's batch cannot flood a vendor.

use std::time::Duration;

use super::{Kind, Registry};

/// A render of a stored project (#534, #541). The compositor and the encoder
/// each use several cores, so two at once is the machine.
pub const RENDER: Kind = Kind {
    name: "render",
    limit: 2,
};

/// A Veo shot: submit, keep the ticket, poll (#537). Minutes of waiting and
/// almost no machine.
pub const VEO_SHOT: Kind = Kind {
    name: "veo_shot",
    limit: 4,
};

/// An ElevenLabs line (#537). Seconds, and mostly network.
pub const SPOKEN_LINE: Kind = Kind {
    name: "spoken_line",
    limit: 4,
};

/// A library item's thumbnail (#535): one decoded frame, quick.
pub const THUMBNAIL: Kind = Kind {
    name: "thumbnail",
    limit: 2,
};

/// A preview proxy (#542): a whole transcode, as heavy as a render.
pub const PROXY: Kind = Kind {
    name: "proxy",
    limit: 1,
};

/// How long a provider job polls before it is [`Stuck`](super::Outcome::Stuck).
///
/// Fifteen minutes. `scorsese_providers::video::WAIT_FOR` is five, and that is
/// right for somebody waiting at a terminal; the maintainer has seen Veo shots
/// take ten, and nobody is waiting on a background worker's patience but the
/// shot. Stuck is not lost — the ticket stays in the row.
pub const PROVIDER_PATIENCE: Duration = Duration::from_secs(15 * 60);

/// Every kind the server runs, each with its handler.
///
/// **Empty until the issues that give a kind something to do**: rendering
/// needs projects stored in Postgres (#534), and a Veo shot or a spoken line
/// pays through `credits::generations` (#537) and needs the library (#535)
/// to put what it made. Each registers its kind here — `.register(RENDER, …)` — and the
/// worker starts claiming it. A kind nothing registers is never claimed, so a
/// job of that kind waits rather than failing.
pub fn registry() -> Registry {
    Registry::new()
}
