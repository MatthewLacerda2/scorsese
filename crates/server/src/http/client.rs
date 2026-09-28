//! Where a request came from: the address a login attempt is counted against
//! (#557).
//!
//! Only two sources are ever believed, and which one is a setting
//! ([`Clients`]), never a guess made per request:
//!
//! - **The connection's own peer** — the default, and all there is when the
//!   server is reached directly, as in development.
//! - **[`CLIENT_HEADER`]**, when the server is told it sits behind the
//!   deploy's nginx (`SCORSESE_TRUST_PROXY`). There the peer is always nginx,
//!   so it says nothing; nginx writes this header on every request it passes
//!   on, replacing whatever the client sent, and the server's port is
//!   published to nothing else.
//!
//! nginx decides what goes in it by **which of its doors the request came
//! through**, and that is what stops an address being forged. Its port 80 is
//! published to nothing and is where the Cloudflare Tunnel points: a request
//! there came from Cloudflare, which puts the client's address in
//! `CF-Connecting-IP` itself, so that header is believed. Everything else —
//! loopback, `tailscale serve`, the LAN — comes in by the port the compose
//! file publishes, where `CF-Connecting-IP` is whatever the client typed, so
//! the connection's own address is used instead. A friend on the tailnet who
//! sends `CF-Connecting-IP: 203.0.113.9` is counted as themselves, and cannot
//! dodge their own limit or run up somebody else's. `deploy/nginx.conf` has
//! the two doors.

use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;

use super::AppState;

/// The header the deploy's nginx names the client's address in.
///
/// Our own name rather than `X-Forwarded-For` or `X-Real-IP`: those are
/// headers a client may send and some proxy may append to, so believing one
/// means knowing exactly which proxies came before. Nobody but our nginx
/// writes this one, and it overwrites it.
pub const CLIENT_HEADER: &str = "x-scorsese-client";

/// Which address to believe for a request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Clients {
    /// The connection's peer: the server is reached directly.
    #[default]
    Peer,
    /// [`CLIENT_HEADER`]: the server is reached only through the deploy's
    /// nginx, which writes it. The peer when the header is missing, which
    /// counts every such request as nginx's — more braking, never less.
    Proxy,
}

/// The client a request came from, if it can be told.
///
/// `None` only on a server run without connection info — a test's router
/// served by hand — where there is no address to count against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientAddress(pub Option<IpAddr>);

impl FromRequestParts<AppState> for ClientAddress {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Infallible> {
        let peer = parts
            .extensions
            .get::<ConnectInfo<SocketAddr>>()
            .map(|ConnectInfo(address)| address.ip());
        let named = || {
            parts
                .headers
                .get(CLIENT_HEADER)?
                .to_str()
                .ok()?
                .trim()
                .parse()
                .ok()
        };
        Ok(Self(match state.clients {
            Clients::Peer => peer,
            Clients::Proxy => named().or(peer),
        }))
    }
}
