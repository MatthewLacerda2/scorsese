//! Where a request came from: the address the access record keeps (#1267,
//! `accounts::access`).
//!
//! **[`VISITOR_HEADER`] first**, which the deploy's nginx writes on every
//! request it passes on, over anything the client sent under that name. nginx
//! decides what goes in it by which of its doors the request came through
//! (`deploy/nginx.conf`): through the tunnel's, Cloudflare's own
//! `CF-Connecting-IP`; through the one compose publishes to loopback, the
//! tailnet and the LAN, the connection's peer — where `CF-Connecting-IP` is
//! whatever the client typed, so it is never read here directly. The server's
//! own port is published to nothing, so in the deploy nothing but nginx can
//! reach it to set the header.
//!
//! **The connection's peer otherwise**: a server reached directly, as in
//! development, where the header is the client's word and the record is
//! nobody's legal obligation.

use std::convert::Infallible;
use std::net::{IpAddr, SocketAddr};

use axum::extract::{ConnectInfo, FromRequestParts};
use axum::http::request::Parts;

/// The header the deploy's nginx names the visitor's address in.
///
/// Our own name rather than `X-Forwarded-For` or `X-Real-IP`: those are
/// headers a client may send and a proxy may append to, so believing one
/// means knowing exactly which proxies came before.
pub const VISITOR_HEADER: &str = "x-scorsese-client";

/// The visitor's address, if it can be told.
///
/// `None` only on a server run without connection info and without nginx in
/// front — a test's router served by hand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Visitor(pub Option<IpAddr>);

impl Visitor {
    /// The visitor of the request `parts` belong to.
    pub fn of(parts: &Parts) -> Self {
        let named = parts
            .headers
            .get(VISITOR_HEADER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse().ok());
        let peer = || {
            parts
                .extensions
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ConnectInfo(address)| address.ip())
        };
        Self(named.or_else(peer))
    }
}

impl<S: Send + Sync> FromRequestParts<S> for Visitor {
    type Rejection = Infallible;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Infallible> {
        Ok(Self::of(parts))
    }
}
