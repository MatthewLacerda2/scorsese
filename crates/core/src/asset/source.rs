//! Where an imported file came from, when a stock library supplied it (#1237).
//!
//! A generated asset carries its whole history — the brief, the model, when
//! and what it cost — but a stock import used to carry only its file name,
//! `assets/pixabay-<id>.<ext>`: who made the picture, its page and the licence
//! it came under were said once in an import's reply and then lost. This block
//! keeps them, in the document, so they travel with the `.scor` folder and
//! every surface — the CLI, the desktop app, the web library — can show them.
//!
//! Only the shape lives here. Which library a file came from and what its
//! licence is called are the importing code's values, never core's
//! (the generality rule): core knows there *is* a library, not what Pixabay's
//! licence says.

use std::fmt;

use serde::{Deserialize, Serialize};

/// The stock library a file was imported from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StockLibrary {
    /// pixabay.com: free footage and photos.
    Pixabay,
}

impl StockLibrary {
    /// Every library, in the order a listing names them.
    pub const ALL: [Self; 1] = [Self::Pixabay];

    /// The library's name as a person reads it: `Pixabay`.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pixabay => "Pixabay",
        }
    }

    /// The library as `project.json` writes it: `pixabay`.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pixabay => "pixabay",
        }
    }
}

impl fmt::Display for StockLibrary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Where an imported `video` or `image` asset came from: the library, its id
/// there, who published it, its page and its licence.
///
/// Bookkeeping, like `created_at`: never hashed, never rendered, never read by
/// anything that decides what a frame looks like. An asset without one is a
/// file the user brought, or a stock file imported before the block existed —
/// its file name is never parsed back into one, because the author and the
/// page are gone and half a record would read as the whole of one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// Which library.
    pub library: StockLibrary,
    /// The library's own id for the file, as text: what its search names it
    /// by. Text rather than a number because a library is free to use any id.
    pub id: String,
    /// Who published it, where the library says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Its page at the library: an `http(s)` address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_url: Option<String>,
    /// The name of the licence it came under, as the library calls it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub licence: Option<String>,
}

/// What is wrong with a [`Source`], when something is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SourceProblem {
    /// No id: nothing would name the file at its library.
    #[error("has an empty `id`")]
    EmptyId,
    /// A page that is not a web address.
    #[error("has a `page_url` that is not an http(s) address")]
    PageNotWeb,
}

impl Source {
    /// The one line a listing prints: `Pixabay 39009, by someone, under the
    /// Pixabay Content License — https://…`, each part only where it is known.
    pub fn says(&self) -> String {
        let mut line = format!("{} {}", self.library, self.id);
        if let Some(author) = &self.author {
            line.push_str(&format!(", by {author}"));
        }
        if let Some(licence) = &self.licence {
            line.push_str(&format!(", under the {licence}"));
        }
        if let Some(page) = &self.page_url {
            line.push_str(&format!(" — {page}"));
        }
        line
    }

    /// The first thing wrong with this block, or `None`.
    pub fn problem(&self) -> Option<SourceProblem> {
        if self.id.trim().is_empty() {
            return Some(SourceProblem::EmptyId);
        }
        let web = |url: &str| url.starts_with("https://") || url.starts_with("http://");
        if self.page_url.as_deref().is_some_and(|url| !web(url)) {
            return Some(SourceProblem::PageNotWeb);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> Source {
        Source {
            library: StockLibrary::Pixabay,
            id: "39009".to_owned(),
            author: Some("someone".to_owned()),
            page_url: Some("https://pixabay.com/videos/id-39009/".to_owned()),
            licence: Some("Pixabay Content License".to_owned()),
        }
    }

    #[test]
    fn a_whole_source_says_every_part_and_is_sound() {
        let whole = source();
        assert_eq!(
            whole.says(),
            "Pixabay 39009, by someone, under the Pixabay Content License — \
             https://pixabay.com/videos/id-39009/"
        );
        assert_eq!(whole.problem(), None);
        let bare = Source {
            author: None,
            page_url: None,
            licence: None,
            ..whole
        };
        assert_eq!(bare.says(), "Pixabay 39009");
        assert_eq!(bare.problem(), None);
    }

    #[test]
    fn an_empty_id_or_a_page_off_the_web_is_a_problem() {
        let blank = Source {
            id: " ".to_owned(),
            ..source()
        };
        assert_eq!(blank.problem(), Some(SourceProblem::EmptyId));
        let local = Source {
            page_url: Some("file:///etc/passwd".to_owned()),
            ..source()
        };
        assert_eq!(local.problem(), Some(SourceProblem::PageNotWeb));
    }

    #[test]
    fn every_library_is_written_as_its_word() {
        for library in StockLibrary::ALL {
            let written = serde_json::to_value(library).expect("a library serialises");
            assert_eq!(written, library.as_str());
        }
    }
}
