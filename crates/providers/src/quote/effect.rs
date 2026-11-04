//! One generated sound effect, as the run would treat it (#1265): priced by
//! the minute of what it asks for, so exact before anything is sent, and
//! still an estimate — the rate is a page somebody copied.

use std::path::Path;

use scorsese_core::Asset;

use super::{Charge, Item};
use crate::prices::{self, dollars, from_cents};
use crate::sound;

/// The quote's line for `asset`, a sound effect.
pub(super) fn effect(root: &Path, asset: &Asset) -> Item {
    let free = |says: String, made: bool| Item {
        subject: asset.id.to_string(),
        says,
        charge: None,
        made,
    };
    match sound::plan(root, asset) {
        sound::Plan::Realized(path) => {
            free(format!("already made — {path} — nothing to pay"), true)
        }
        sound::Plan::Unready(why) => free(format!("not yet — {why}"), false),
        sound::Plan::Submit => match sound::Brief::of(asset) {
            Ok(brief) => {
                let priced = prices::sound(brief.millis);
                Item {
                    subject: asset.id.to_string(),
                    says: format!(
                        "{} — {} seconds of sound",
                        dollars(from_cents(priced.cents), 2),
                        f64::from(priced.millis) / 1000.0
                    ),
                    charge: Some(Charge {
                        brief: brief.digest(),
                        cents: priced.cents,
                    }),
                    made: false,
                }
            }
            Err(why) => free(format!("not yet — {why}"), false),
        },
    }
}
