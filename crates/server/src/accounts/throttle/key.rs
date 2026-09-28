//! What an attempt is counted against: the email it named, and the address it
//! came from.

use std::fmt;
use std::net::{IpAddr, Ipv6Addr};

/// The longest key kept: an email address's own limit. A login may send a
/// longer string, and it is counted under its first 254 characters rather than
/// letting anyone write arbitrarily long rows.
const LONGEST: usize = 254;

/// One counter's name.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Key {
    /// The address a client connected from — see [`Key::address`].
    Address(String),
    /// The email a login named, normalised as the login normalises it, whether
    /// or not anybody has that account.
    Email(String),
}

impl Key {
    /// The email `raw` names, trimmed and lower-cased like
    /// [`normalize_email`](crate::accounts::normalize_email) — but for any
    /// string at all, because a malformed email is braked like any other.
    pub fn email(raw: &str) -> Self {
        Self::Email(raw.trim().to_lowercase().chars().take(LONGEST).collect())
    }

    /// The counter for a client at `address`.
    ///
    /// IPv4 as it is. IPv6 by the `/64` it is in, because that is what one
    /// subscriber is handed: counting each address separately would give a
    /// single home eighteen quintillion fresh counters to rotate through.
    pub fn address(address: IpAddr) -> Self {
        Self::Address(match address {
            IpAddr::V4(v4) => v4.to_string(),
            IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
                Some(v4) => v4.to_string(),
                None => format!("{}/64", network(v6)),
            },
        })
    }

    /// The key the operator means by `text`: an address when it reads as one
    /// (an IPv6 `/64` as `user locks` prints it included), an email otherwise.
    pub fn parse(text: &str) -> Self {
        let text = text.trim();
        let bare = text.strip_suffix("/64").unwrap_or(text);
        match bare.parse::<IpAddr>() {
            Ok(address) => Self::address(address),
            Err(_) => Self::email(text),
        }
    }

    /// The `kind` column's value.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Address(_) => "address",
            Self::Email(_) => "email",
        }
    }

    /// The `key` column's value.
    pub fn text(&self) -> &str {
        match self {
            Self::Address(text) | Self::Email(text) => text,
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.kind(), self.text())
    }
}

/// `address` with everything after its first 64 bits zeroed.
fn network(address: Ipv6Addr) -> Ipv6Addr {
    let bits = u128::from(address) & !(u128::from(u64::MAX));
    Ipv6Addr::from(bits)
}
