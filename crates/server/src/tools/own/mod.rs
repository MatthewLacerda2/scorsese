//! The server's own tools: the ones where a hosted project genuinely works
//! differently from a folder, and one that has no local meaning at all.
//!
//! Each is described in its module the way a registry tool describes itself —
//! a name, a description whose first sentence stands alone, and a schema in
//! which every argument says what it is — and the same test holds them to it
//! (`tests/tools/described.rs`).

mod catalogue;
mod projects;
mod queue;
mod templates;

use scorsese_mcp::Reply;
use serde_json::{Value, json};

use super::Caller;
use crate::credits::{history, tool as spending};

/// One of the server's own tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Own {
    /// The caller's projects, with their ids.
    ProjectList,
    /// A new stored project.
    ProjectNew,
    /// The caller's library.
    Library,
    /// Library files into a project.
    Import,
    /// A render, made by the queue.
    Render,
    /// Where the caller's jobs are.
    Jobs,
    /// Generations, paid from credits.
    Generate,
    /// What the caller has spent.
    Spending,
    /// The caller's templates.
    TemplateList,
    /// Clips of a project, saved as a template.
    TemplateSave,
    /// A template, copied into a project.
    TemplateInsert,
}

impl Own {
    /// Every one of them.
    pub(crate) const ALL: [Self; 11] = [
        Self::ProjectList,
        Self::ProjectNew,
        Self::Library,
        Self::Import,
        Self::Render,
        Self::Jobs,
        Self::Generate,
        Self::Spending,
        Self::TemplateList,
        Self::TemplateSave,
        Self::TemplateInsert,
    ];

    /// The ones listed after the registry's tools, standing in for none.
    pub(crate) const LAST: [Self; 4] = [
        Self::TemplateList,
        Self::TemplateSave,
        Self::TemplateInsert,
        Self::Spending,
    ];

    /// The tool called `name`.
    pub(crate) fn named(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|own| own.name() == name)
    }

    /// What stands where the registry's `name` stands in the list.
    pub(crate) fn replacing(name: &str) -> &'static [Self] {
        match name {
            "project_new" => &[Self::ProjectList, Self::ProjectNew],
            "import" => &[Self::Library, Self::Import],
            "generate" => &[Self::Generate],
            "render" => &[Self::Render, Self::Jobs],
            _ => &[],
        }
    }

    /// How a client names it.
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::ProjectList => projects::LIST,
            Self::ProjectNew => projects::NEW,
            Self::Library => catalogue::LIBRARY,
            Self::Import => catalogue::IMPORT,
            Self::Render => queue::RENDER,
            Self::Jobs => queue::JOBS,
            Self::Generate => super::generate::NAME,
            Self::Spending => spending::NAME,
            Self::TemplateList => templates::LIST,
            Self::TemplateSave => templates::SAVE,
            Self::TemplateInsert => templates::INSERT,
        }
    }

    /// What it does, in the words a client is shown.
    fn description(self) -> &'static str {
        match self {
            Self::ProjectList => projects::LIST_SAYS,
            Self::ProjectNew => projects::NEW_SAYS,
            Self::Library => catalogue::LIBRARY_SAYS,
            Self::Import => catalogue::IMPORT_SAYS,
            Self::Render => queue::RENDER_SAYS,
            Self::Jobs => queue::JOBS_SAYS,
            Self::Generate => super::generate::DESCRIPTION,
            Self::Spending => spending::DESCRIPTION,
            Self::TemplateList => templates::LIST_SAYS,
            Self::TemplateSave => templates::SAVE_SAYS,
            Self::TemplateInsert => templates::INSERT_SAYS,
        }
    }

    /// The JSON Schema of its arguments.
    fn schema(self) -> Value {
        match self {
            Self::ProjectList => json!({ "type": "object", "properties": {} }),
            Self::ProjectNew => projects::new_schema(),
            Self::Library => catalogue::library_schema(),
            Self::Import => catalogue::import_schema(),
            Self::Render => queue::render_schema(),
            Self::Jobs => queue::jobs_schema(),
            Self::Generate => super::generate::schema(),
            Self::Spending => spending::schema(),
            Self::TemplateList => json!({ "type": "object", "properties": {} }),
            Self::TemplateSave => templates::save_schema(),
            Self::TemplateInsert => templates::insert_schema(),
        }
    }

    /// As `tools/list` shows it.
    pub(crate) fn listing(self) -> Value {
        json!({
            "name": self.name(),
            "description": self.description(),
            "inputSchema": self.schema()
        })
    }

    /// Run it for `caller`.
    pub(crate) async fn call(
        self,
        caller: &Caller<'_>,
        arguments: &Value,
    ) -> Result<Reply, String> {
        match self {
            Self::ProjectList => projects::list(caller).await,
            Self::ProjectNew => projects::new(caller, arguments).await,
            Self::Library => catalogue::library(caller, arguments).await,
            Self::Import => catalogue::import(caller, arguments).await,
            Self::Render => queue::render(caller, arguments).await,
            Self::Jobs => queue::jobs(caller, arguments).await,
            Self::Generate => super::generate::call(caller, arguments).await,
            Self::Spending => spending_history(caller, arguments).await,
            Self::TemplateList => templates::list(caller).await,
            Self::TemplateSave => templates::save(caller, arguments).await,
            Self::TemplateInsert => templates::insert(caller, arguments).await,
        }
    }
}

/// `spending_history`: the ledger, read for the caller (#537).
async fn spending_history(caller: &Caller<'_>, arguments: &Value) -> Result<Reply, String> {
    let filter = spending::filter(arguments).map_err(|error| error.to_string())?;
    let seen = history::read(&caller.toolbox.pool, caller.user, &filter)
        .await
        .map_err(|error| match error {
            crate::credits::CreditError::Invalid(why) => why,
            other => super::database(other),
        })?;
    Ok(spending::answer(&seen).into())
}
