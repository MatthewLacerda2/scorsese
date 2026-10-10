//! Where the page renderer comes from when a command first needs one.

use scorsese_providers::chromium;
use scorsese_render::page::{self, Supply};

/// Lets a render download the page renderer the first time an `html` clip
/// needs drawing (#776), saying so in one line on stderr — so `stdout` stays
/// what the command answers, and a person running it once sees why it paused.
pub(crate) fn supply() {
    page::supply(Supply::new(chromium::installed, || {
        chromium::fetch(&mut |at| {
            if at.received == 0 {
                eprintln!("{}", at.starting());
            }
        })
        .map_err(|error| error.to_string())
    }));
}
