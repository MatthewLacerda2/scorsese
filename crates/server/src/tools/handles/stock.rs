//! Where stock media comes from, within each member's share, and the test
//! seam that replaces the vendors.

use std::sync::Arc;

use scorsese_mcp::{Libraries, Stock};
use scorsese_providers::credentials::{Provider, resolve};
use scorsese_providers::stock::{self, PixabayLibrary};

use super::super::Toolbox;
use crate::db::UserId;

impl Toolbox {
    /// The same tools searching and importing stock media of every kind from
    /// `library` instead of Pixabay and LottieFiles — a test's, which needs no
    /// key and no network.
    pub fn stocked_from(mut self, library: Stock) -> Self {
        self.handles.stock = Some(library);
        self
    }

    /// Where `user`'s stock tools find each medium's library: the toolbox's
    /// own when it was given one, and otherwise each medium's
    /// (`stock::library`), every one within `user`'s share.
    pub(in crate::tools) fn libraries(&self, user: UserId) -> Libraries {
        let (given, shares) = (self.handles.stock.clone(), self.handles.shares.clone());
        Arc::new(move |medium| {
            let library = match &given {
                Some(library) => library.clone(),
                None => stock::library(medium)
                    .map(Arc::from)
                    .map_err(|error| format!("{error}"))?,
            };
            Ok(shares.of(user, library))
        })
    }

    /// The library `user`'s pick of footage or a photo is imported from,
    /// within their share of it (`share`): the toolbox's own when it was
    /// given one, and otherwise Pixabay, keyed from the one resolver. Why
    /// there is none, in words for the model, which tells the person.
    pub(in crate::tools) fn footage(&self, user: UserId) -> Result<Stock, String> {
        let shares = &self.handles.shares;
        if let Some(library) = &self.handles.stock {
            return Ok(shares.of(user, library.clone()));
        }
        match resolve(Provider::Pixabay) {
            Ok(key) => Ok(shares.of(user, Arc::new(PixabayLibrary::new(&key.secret)))),
            Err(error) => {
                tracing::error!("stock import: {error}");
                Err(format!(
                    "{} is not set on this server, so stock media cannot be brought in",
                    Provider::Pixabay.variable()
                ))
            }
        }
    }
}
