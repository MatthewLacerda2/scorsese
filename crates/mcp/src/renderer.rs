//! The page renderer, fetched the first time a tool draws a page (#776).
//!
//! The server's own binary turns this on ([`fetch_on_first_use`]); the hosted
//! server, which runs the same tools, does not — its browser is the image's
//! (#778), and it never downloads one.
//!
//! The log hears the download start. The reply hears it finish: the render
//! that fetched it carries `Note::PageRendererFetched`, which `render` and
//! `still` pass on with the rest of a render's notes — the agent is who should
//! know a call paused for a 100 MB download, and that it will not again.

use scorsese_providers::chromium;
use scorsese_render::page::{self, Supply};

/// Lets this server's renders download the page renderer on first use.
pub fn fetch_on_first_use() {
    page::supply(Supply::new(chromium::installed, || {
        chromium::fetch(&mut |at| {
            if at.received == 0 {
                eprintln!("{}", at.starting());
            }
        })
        .map_err(|error| error.to_string())
    }));
}
