//! `POST /api/ab/events` (#1553): the landing page telling the server which
//! versions of its tests a visitor was shown, and what they did
//! ([`crate::ab`]).
//!
//! Anyone may post: the visitor is signed out by definition. So the route
//! guards itself: a body of at most [`CAP`] bytes, a batch of the shape a
//! client of ours sends ([`Batch::valid`]), at most [`ROWS_PER_ADDRESS_PER_DAY`]
//! rows from one address ([`Visitor`], an IPv6 one counted by its /64, which
//! one machine holds whole) in a day, [`ROWS_PER_DAY`] from everybody, and
//! the per-visitor caps [`crate::ab`] holds. Whatever it refuses, and whatever fails, it answers
//! `204` all the same: the page never needs to know, and a script probing it
//! learns nothing.
//!
//! Where the visitor is comes from `X-Scorsese-Country` and
//! `X-Scorsese-Region`, which the deploy's nginx writes from Cloudflare's
//! location headers on the tunnel's door only, and blanks on any other
//! (`deploy/nginx.conf`, the same rule as the address).

use std::net::{IpAddr, Ipv6Addr};
use std::time::Duration;

use axum::Json;
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::{HeaderMap, StatusCode};

use super::AppState;
use super::visitor::Visitor;
use crate::ab::{self, Batch, Place, VISITOR_COOKIE};

/// The most a batch may weigh, in bytes.
pub const CAP: usize = 4 * 1024;

/// Rows one address may write in a day: an office or a phone carrier's
/// shared address is many visitors.
pub const ROWS_PER_ADDRESS_PER_DAY: u32 = 2000;

/// Rows everybody together may write in a day: a hundred times a busy day's
/// visits, and the bound whatever addresses a client rotates through.
pub const ROWS_PER_DAY: u32 = 100_000;

/// The day the counts run over.
pub const DAY: Duration = Duration::from_secs(24 * 60 * 60);

/// The header nginx names the visitor's country in.
pub const COUNTRY_HEADER: &str = "x-scorsese-country";

/// The header nginx names the visitor's region in.
pub const REGION_HEADER: &str = "x-scorsese-region";

/// `POST /api/ab/events`: record the batch, answer `204` whatever happens.
pub async fn events(
    State(state): State<AppState>,
    Visitor(address): Visitor,
    headers: HeaderMap,
    batch: Result<Json<Batch>, JsonRejection>,
) -> StatusCode {
    let Ok(Json(batch)) = batch else {
        return StatusCode::NO_CONTENT;
    };
    let rows = batch.rows();
    if !batch.valid()
        || state.ab_rows.take(network(address), rows).is_err()
        || state.ab_total.take((), rows).is_err()
    {
        return StatusCode::NO_CONTENT;
    }
    let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
    let place = Place::new(header(COUNTRY_HEADER), header(REGION_HEADER));
    if let Err(error) = ab::record(&state.pool, &batch, &place).await {
        tracing::error!("could not record landing events: {error}");
    }
    StatusCode::NO_CONTENT
}

/// What the per-address count is keyed by: an IPv4 address, or an IPv6
/// address's /64.
pub fn network(address: Option<IpAddr>) -> Option<IpAddr> {
    address.map(|address| match address {
        IpAddr::V4(_) => address,
        IpAddr::V6(v6) => IpAddr::V6(Ipv6Addr::from(v6.to_bits() & !u128::from(u64::MAX))),
    })
}

/// The visitor code the request's `scorsese_visitor` cookie carries.
pub fn visitor_cookie(headers: &HeaderMap) -> Option<String> {
    super::auth::cookie(headers, VISITOR_COOKIE).and_then(ab::visitor_code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header::COOKIE;

    #[test]
    fn an_ipv6_address_is_counted_by_its_64() {
        let one = |text: &str| network(Some(text.parse().unwrap()));
        assert_eq!(one("2001:db8:1:2:aaaa::1"), one("2001:db8:1:2:ffff::9"));
        assert_ne!(one("2001:db8:1:2::1"), one("2001:db8:1:3::1"));
        assert_eq!(one("2001:db8:1:2:aaaa::1"), one("2001:db8:1:2::"));
        assert_eq!(one("198.51.100.4"), Some("198.51.100.4".parse().unwrap()));
        assert_eq!(network(None), None);
    }

    #[test]
    fn the_visitor_cookie_is_found_among_others() {
        let mut headers = HeaderMap::new();
        headers.append(COOKIE, "theme=dark".parse().unwrap());
        headers.append(
            COOKIE,
            "scorsese_session=x; scorsese_visitor=0B7F6A3E-9C1D-4E2F-8A5B-6C7D8E9F0A1B"
                .parse()
                .unwrap(),
        );
        assert_eq!(
            visitor_cookie(&headers).as_deref(),
            Some("0b7f6a3e-9c1d-4e2f-8a5b-6c7d8e9f0a1b")
        );
        let mut forged = HeaderMap::new();
        forged.append(COOKIE, "scorsese_visitor=drop table".parse().unwrap());
        assert_eq!(visitor_cookie(&forged), None);
    }
}
