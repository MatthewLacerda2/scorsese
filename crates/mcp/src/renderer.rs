//! The page renderer, fetched the first time a tool draws a page (#776), and
//! said in the reply that follows.
//!
//! The server's own binary turns this on ([`fetch_on_first_use`]); the hosted
//! server, which runs the same tools, does not — its browser is the image's
//! (#778), and it never downloads one.
//!
//! Said in a reply rather than only in the log, because the agent is who
//! should know a call paused for a 100 MB download and will not again. Which
//! reply: the first one after the download finishes — the call that drew the
//! page, or for a render that runs on after its call has answered, the next.

use std::sync::{Mutex, PoisonError};

use scorsese_providers::chromium;
use scorsese_render::page::{self, Supply};

/// What to say in the next reply, once a download has finished.
static UNSAID: Mutex<Option<String>> = Mutex::new(None);

/// Lets this server's renders download the page renderer on first use.
pub fn fetch_on_first_use() {
    page::supply(Supply::new(chromium::installed, || {
        // The log hears it start; the reply hears it finish.
        let fetched = chromium::fetch(&mut |at| {
            if at.received == 0 {
                eprintln!("{}", at.starting());
            }
        });
        if let Ok(binary) = &fetched {
            let said = format!(
                "scorsese fetched its page renderer (a headless browser that draws html \
                 clips), once — kept at {} for every project from now on",
                binary.display()
            );
            *UNSAID.lock().unwrap_or_else(PoisonError::into_inner) = Some(said);
        }
        fetched.map_err(|error| error.to_string())
    }));
}

/// The download to mention, if one finished since the last reply said so.
pub(crate) fn unsaid() -> Option<String> {
    UNSAID.lock().unwrap_or_else(PoisonError::into_inner).take()
}
