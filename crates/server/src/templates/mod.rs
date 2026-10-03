//! A user's templates, stored in Postgres (#546): pieces of an edit — an
//! intro, an outro, a running gag — saved from one project to be copied into
//! any other of the same user's.
//!
//! ## A template is a document, like a project
//!
//! What is stored is what `scorsese_core::template::extract` makes: a
//! `project.json` document holding only the saved clips, their tracks and the
//! assets they show. So it is the same kind of row a project is — a `JSONB`
//! column read into [`scorsese_core::Project`], a `name` generated from it —
//! and nothing here knows what a clip is: saving is `core`'s extract, inserting
//! is `core`'s insert, and this module only keeps the rows (`docs/web.md`, *The
//! edit is a document*). It also means a schema bump carries every stored
//! template forward with the same `scorsese_core::migrate` steps, on start, in
//! [`migrate_stored`].
//!
//! **Copy, never link** (#527): inserting copies the clips into the project,
//! so changing or deleting a template never changes a video it was used in.
//! The files are not copied — a template names library files by hash, exactly
//! as a project does, so a Veo intro generated once is reused for nothing.
//!
//! ## Which files a template uses
//!
//! `template_assets` is `project_assets` for templates: one row per distinct
//! `sha256` in the document, rewritten with every save and never any other
//! way, with a key to `library_items`. So a library file a template uses cannot
//! be deleted — the library refuses, naming the template — for the reason a
//! project's file cannot: the next insertion would name a file that is gone.
//!
//! **Names are unique per user, whatever their case**, because the way a
//! person asks for one is by name — "use my intro". Saving under a name that is
//! taken is refused unless the caller says to replace it; replacing is how a
//! template is updated, and it cannot reach a video the old one went into.
//!
//! ## What a template is for
//!
//! A template carries a **description** (#560) — prose saying what it is for,
//! so "my usual intro" can be told from the outro before anything is inserted.
//! It is a column beside the document, not a field in it: the document is a
//! `project.json` document, and `scorsese_core::template::Description` has why
//! a field there would be a format change for something no project has.
//! Replacing a template without saying a new one keeps the old description —
//! an update to the clips is not a reason to forget what they are for.

mod store;

pub use store::{delete, list, migrate_stored, open, save};

use scorsese_core::migrate::MigrateError;
use scorsese_core::template::Description;
use scorsese_core::{LoadError, Project};
use serde::Serialize;

/// A template as a list shows it.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Summary {
    /// What to insert it by.
    pub id: i64,
    /// Its name — unique among the user's templates, ignoring case.
    pub name: String,
    /// How long it runs, from its first clip's start to its last clip's end.
    pub seconds: f64,
    /// How many clips it holds.
    pub clips: usize,
    /// On how many tracks.
    pub tracks: usize,
    /// The assets its clips show, by id — what a person or an assistant reads
    /// to tell one template from another.
    pub assets: Vec<String>,
    /// What it is for, in the words it was saved with — `None` when nobody
    /// said. Kept beside the document, not in it (`template::Description`).
    pub description: Option<Description>,
    /// When it was last saved, in seconds since the Unix epoch.
    pub updated_at: i64,
}

impl Summary {
    fn of(id: i64, updated_at: i64, template: &Project, description: Option<Description>) -> Self {
        let end = template
            .clips()
            .map(|(_, clip)| clip.end())
            .max()
            .unwrap_or_default();
        Self {
            id,
            name: template.name.clone(),
            seconds: template.timeline_fps.seconds(end),
            clips: template.clips().count(),
            tracks: template.tracks.len(),
            assets: template.assets.iter().map(|a| a.id.to_string()).collect(),
            description,
            updated_at,
        }
    }
}

/// A template opened to be inserted.
#[derive(Debug, Clone)]
pub struct Stored {
    /// Everything a list shows.
    pub summary: Summary,
    /// The fragment itself.
    pub document: Project,
}

/// Why a template operation did not happen.
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    /// The user has no template by that id — including when somebody else
    /// does, which is indistinguishable on purpose.
    #[error("there is no such template of yours")]
    NotFound,

    /// A template of the user's already has this name.
    #[error(
        "you already have a template called \u{201c}{name}\u{201d} (id {id}); replace it, or \
         choose another name"
    )]
    NameTaken {
        /// The template that has it.
        id: i64,
        /// Its name as stored.
        name: String,
    },

    /// A template needs a name to be asked for by.
    #[error("a template needs a name")]
    Unnamed,

    /// The document names files its owner's library does not hold.
    #[error("the template names files that are not in your library: {}", .0.join(", "))]
    UnknownFiles(Vec<String>),

    /// A stored document does not read as a template.
    #[error("stored template {id} does not load: {source}")]
    Unreadable {
        /// Which template.
        id: i64,
        /// Why.
        #[source]
        source: LoadError,
    },

    /// A stored document could not be carried forward to this build.
    #[error("stored template {id} could not be migrated: {source}")]
    Migrate {
        /// Which template.
        id: i64,
        /// Why.
        #[source]
        source: MigrateError,
    },

    /// The document could not be serialised.
    #[error("serialising the template: {0}")]
    Serialize(#[from] serde_json::Error),

    /// The database refused or could not be reached.
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
}
