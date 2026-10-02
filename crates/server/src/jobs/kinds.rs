//! The kinds of job the server runs, how many of each at once, and the
//! registry their handlers are plugged into.
//!
//! The limits are for the machine this runs on: four cores and 16 GB, shared
//! with Postgres and the other containers. What uses the CPU is kept to a few
//! at a time; what waits on a provider's servers barely uses the machine and
//! is limited only so one person's batch cannot flood a vendor.

use std::sync::Arc;
use std::time::Duration;

use super::{Kind, Registry};
use crate::Files;
use crate::generations::{self, Keys, Timing, Vendors};
use crate::library::{proxy, thumbnail};
use crate::renders::job as render;

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

/// A Gemini still (#461). Seconds, mostly network, like a line.
pub const STILL_IMAGE: Kind = Kind {
    name: "still_image",
    limit: 4,
};

/// A **preview** render (#542): the cut, small, for the editor to play. Its
/// own kind so previews queue behind each other and never take a slot a
/// finished render is waiting for — one at a time, because a preview is
/// superseded by the next edit anyway (`renders::job`).
pub const PREVIEW: Kind = Kind {
    name: "preview",
    limit: 1,
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

/// The kinds whose owner may stop them (#660): the two that take the
/// machine's cores for as long as a project is long.
///
/// A paid generation is not here: the provider bills whether or not anybody
/// still wants the shot, and the reservation must settle on its answer. A
/// thumbnail or a proxy is the server's own housekeeping, quick or needed by
/// the next preview, and nobody asked for it to stop.
pub const STOPPABLE: [Kind; 2] = [RENDER, PREVIEW];

/// Whether a job of the kind called `name` may be stopped by its owner.
pub fn stoppable(name: &str) -> bool {
    STOPPABLE.iter().any(|kind| kind.name == name)
}

/// How long a provider job polls before it is [`Stuck`](super::Outcome::Stuck).
///
/// Fifteen minutes. `scorsese_providers::video::WAIT_FOR` is five, and that is
/// right for somebody waiting at a terminal; the maintainer has seen Veo shots
/// take ten, and nobody is waiting on a background worker's patience but the
/// shot. Stuck is not lost — the ticket stays in the row.
pub const PROVIDER_PATIENCE: Duration = Duration::from_secs(15 * 60);

/// Every kind the server runs, each with its handler, drawing on `files` —
/// and, for the paid generations, on the real vendors, with their keys from
/// the server's environment ([`Keys`]).
///
/// [`THUMBNAIL`] (#535), [`RENDER`] (#541), [`VEO_SHOT`] and [`SPOKEN_LINE`]
/// (#539, `crate::generations`), [`STILL_IMAGE`] (#461), [`PREVIEW`] and [`PROXY`] (#542) have theirs.
/// A kind nothing registers is never claimed, so a job of that kind waits
/// rather than failing.
pub fn registry(files: &Files) -> Registry {
    with_vendors(files, Arc::new(Keys), Timing::default())
}

/// [`registry`], with the paid generations reaching `vendors` and waiting as
/// `timing` says — for a test, whose vendors never spend a cent.
pub fn with_vendors(files: &Files, vendors: Arc<dyn Vendors>, timing: Timing) -> Registry {
    Registry::new()
        .register(
            THUMBNAIL,
            thumbnail::handler(files.storage.clone(), files.tools.clone()),
        )
        .register(
            RENDER,
            render::handler(
                files.renders.clone(),
                files.tools.clone(),
                files.storage.clone(),
            ),
        )
        .register(
            PREVIEW,
            render::handler(
                files.renders.clone(),
                files.tools.clone(),
                files.storage.clone(),
            ),
        )
        .register(
            PROXY,
            proxy::handler(files.storage.clone(), files.tools.clone()),
        )
        .register(
            VEO_SHOT,
            generations::shot_handler(
                files.storage.clone(),
                files.tools.clone(),
                Arc::clone(&vendors),
                timing,
            ),
        )
        .register(
            STILL_IMAGE,
            generations::still_handler(
                files.storage.clone(),
                files.tools.clone(),
                Arc::clone(&vendors),
            ),
        )
        .register(
            SPOKEN_LINE,
            generations::line_handler(files.storage.clone(), files.tools.clone(), vendors),
        )
}
