//! scorsese's tools, served to one signed-in user at a time (#539): what web
//! MCP answers `tools/list` and `tools/call` with, and what the built-in
//! assistant (#540) calls in-process.
//!
//! ## One registry, two ways to address a project
//!
//! `scorsese-mcp`'s tools take a **project directory**. A web user's projects
//! are **rows** — a `project.json` document in Postgres, its files in their
//! library. The two meet by laying the row out as a directory for the length
//! of one call (`folder`): the document written, every file it names linked
//! by hash from the user's own library, the registry's tool run on it
//! unchanged, and the document — with the recipes and script it keeps beside
//! it (#560) — read back and saved through
//! [`projects::save_with_files`](crate::projects::save_with_files) with its
//! revision check (`stored`). So every edit the web makes is the edit the CLI and the stdio
//! server make, by the same code, and nothing here knows what a clip is.
//!
//! What changes is the argument: on the web `project` is **the id** of one of
//! the caller's projects, and the schema a client is shown says so. Every
//! tool's own description is the registry's, word for word; `surface`
//! decides, tool by tool, whether a registry tool is served as it is, served
//! with its file arguments held inside the project, replaced by a web tool of
//! the same name, or not served yet — and a test holds every registry tool to
//! exactly one of those answers, so a tool added to `scorsese-mcp` cannot reach
//! the web by accident, nor be left off it without a reason written down.
//!
//! ## Tools that need the database are the server's own
//!
//! The recorded tension (`lib.rs`, *The tool registry is `scorsese-mcp`'s*)
//! was that the `Tool` trait is synchronous and knows nothing of users, so the
//! day a tool needs the database is the day to move the registry down a layer.
//! That day came with `spending_history`, `project_list`, and a `generate` that
//! pays through credits — and the answer turned out to be **no move**. The
//! tools that need Postgres have no local meaning at all: a `.scor` folder has
//! no ledger, no project list, no queue. Moving the registry beneath both
//! crates would put Postgres-shaped tools in a crate the stdio server links,
//! for a binary that could never call them. So the web surface is the
//! registry's tools **plus** the server's own (`own`, `generate`), listed
//! together under one set of rules — every tool and argument described, held
//! by a test as `docs/mcp.md` holds the registry — and `scorsese-mcp` still
//! never learns about users.
//!
//! The server's own tools are the ones where a hosted project genuinely works
//! differently: making a project is a row, not a directory; importing is
//! bringing a library file in, not copying a path from this machine; a render
//! is a job whose file is downloaded; and a generation is paid for from the
//! user's credits and made by the job queue. Each still reaches its effect
//! through the lower crates — `core` adds the asset, `providers` quotes and
//! realises, `render` renders — so the rule that the server has no editing
//! logic of its own holds for them too.
//!
//! ## Every call is recorded
//!
//! In `tool_calls` (`log`), with who made it: the user's own client over web
//! MCP, or the built-in assistant. The same table, so "what did anything do to
//! my project?" has one answer.

mod bakes;
mod carried;
mod design;
mod fetched;
mod folder;
mod generate;
mod log;
mod own;
mod picked;
mod quotes;
mod size;
mod stored;
mod surface;

pub use fetched::keep as keep_fetched;
pub use folder::{Folder, lay_out};
pub use log::Client;
pub use quotes::Pending;

use scorsese_mcp::{Reply, Tool};
use scorsese_render::{Cancel, Tools};
use serde_json::Value;
use sqlx::postgres::PgPool;

use crate::db::UserId;
use crate::jobs::Queue;
use crate::library::Library;
use crate::renders::RenderCache;
use crate::storage::Storage;

/// Everything a tool call may reach, for whichever user makes it. Cheap to
/// clone — every field is a handle.
#[derive(Clone)]
pub struct Toolbox {
    pool: PgPool,
    library: Library,
    storage: Storage,
    tools: Tools,
    queue: Queue,
    renders: RenderCache,
}

impl Toolbox {
    /// Tools answering from `pool` (the member pool), with users' files in
    /// `library` and its `storage`, ffmpeg in `tools`, jobs announced on
    /// `queue` and renders kept in `renders`.
    pub fn new(
        pool: PgPool,
        library: Library,
        tools: Tools,
        queue: Queue,
        renders: RenderCache,
    ) -> Self {
        Self {
            storage: library.storage().clone(),
            pool,
            library,
            tools,
            queue,
            renders,
        }
    }

    /// Every tool a user is offered, as `tools/list` shows it — name,
    /// description and input schema.
    pub fn listing(&self) -> Vec<Value> {
        surface::listing()
    }

    /// Whether there is a tool called `name` here.
    pub fn serves(&self, name: &str) -> bool {
        surface::find(name).is_some()
    }

    /// Run the tool called `name` for `user`, on their projects and their
    /// library only, and record the call as `client`'s.
    ///
    /// The `Err` string is a refusal, in words for whoever asked — MCP's
    /// `isError`, the same shape a stdio tool refuses in. A tool that does not
    /// exist is refused the same way; a transport that answers that
    /// differently asks [`Toolbox::serves`] first.
    pub async fn call(
        &self,
        user: UserId,
        client: Client,
        name: &str,
        arguments: &Value,
    ) -> Result<Reply, String> {
        self.recorded(user, client, name, arguments).await.1
    }

    /// [`Toolbox::call`], stopped when `cancel` is tripped — web MCP's answer
    /// to `notifications/cancelled` (#660). A registry tool is handed it
    /// through `Tool::call_cancellable`, exactly as the stdio server hands it
    /// over, and a call cancelled while it waits on a moving project is not
    /// run again.
    pub async fn call_cancellable(
        &self,
        user: UserId,
        client: Client,
        name: &str,
        arguments: &Value,
        cancel: &Cancel,
    ) -> Result<Reply, String> {
        let (_, outcome) = self
            .run(user, client, name, arguments, None, cancel.clone())
            .await;
        outcome.map_err(|refusal| refusal.to_string())
    }

    /// [`Toolbox::call`], and the call's row in `tool_calls` — `None` for a
    /// tool that does not exist, which is refused before anything is written.
    pub async fn recorded(
        &self,
        user: UserId,
        client: Client,
        name: &str,
        arguments: &Value,
    ) -> (Option<i64>, Result<Reply, String>) {
        let (id, outcome) = self
            .run(user, client, name, arguments, None, Cancel::new())
            .await;
        (id, outcome.map_err(|refusal| refusal.to_string()))
    }

    /// A call from the web editor (#545), recorded as [`Client::Editor`], made
    /// against revision `at` of the project when it names one: then it is
    /// refused as [`Refusal::Moved`] — not run again — if the project is at any
    /// other revision when the tool reads it or saves, because the edit was
    /// worked out on what the user saw.
    pub async fn edit(
        &self,
        user: UserId,
        name: &str,
        arguments: &Value,
        at: Option<i64>,
    ) -> Result<Reply, Refusal> {
        self.run(user, Client::Editor, name, arguments, at, Cancel::new())
            .await
            .1
    }

    /// Run a call and record it — `None` for a tool that does not exist.
    async fn run(
        &self,
        user: UserId,
        client: Client,
        name: &str,
        arguments: &Value,
        at: Option<i64>,
        cancel: Cancel,
    ) -> (Option<i64>, Result<Reply, Refusal>) {
        let Some(entry) = surface::find(name) else {
            return (None, Err(format!("there is no tool `{name}`").into()));
        };
        let id = match log::begin(&self.pool, user, client, name, arguments).await {
            Ok(id) => id,
            Err(error) => return (None, Err(database(error).into())),
        };
        let caller = Caller {
            toolbox: self,
            user,
            call: id,
            at,
            cancel,
        };
        let outcome = match entry {
            surface::Entry::Shared(tool, serve) => {
                stored::run(&caller, tool.as_ref(), serve, arguments).await
            }
            surface::Entry::Own(own) => own.call(&caller, arguments).await.map_err(Refusal::Said),
        };
        let said = outcome.as_ref().map_err(ToString::to_string);
        log::end(&self.pool, user, id, said).await;
        (Some(id), outcome)
    }

    /// Record a call of `name` that is refused before it runs, for `why` —
    /// what the assistant does with a call it will not make on the model's
    /// word (`crate::assistant`). The refusal, as [`Toolbox::call`] words one.
    pub async fn refuse(
        &self,
        user: UserId,
        client: Client,
        name: &str,
        arguments: &Value,
        why: String,
    ) -> Result<Reply, String> {
        let refused = Err(why);
        match log::begin(&self.pool, user, client, name, arguments).await {
            Ok(id) => log::end(&self.pool, user, id, refused.as_ref().map_err(Clone::clone)).await,
            Err(error) => eprintln!("scorsese-server: could not record a refused call: {error}"),
        }
        refused
    }

    /// The quote tool call `call` of `user`'s issued, if it is still unspent.
    pub async fn pending_quote(
        &self,
        user: UserId,
        call: i64,
    ) -> Result<Option<Pending>, sqlx::Error> {
        let mut tx = crate::db::scoped(&self.pool, user).await?;
        let pending = quotes::issued_by(&mut tx, call).await?;
        tx.commit().await?;
        Ok(pending)
    }

    /// Forget `user`'s quote `token` unspent.
    pub async fn withdraw_quote(&self, user: UserId, token: &str) -> Result<(), sqlx::Error> {
        let mut tx = crate::db::scoped(&self.pool, user).await?;
        quotes::withdraw(&mut tx, token).await?;
        tx.commit().await
    }
}

/// Who a tool is running for, and with what.
pub(crate) struct Caller<'a> {
    toolbox: &'a Toolbox,
    user: UserId,
    /// This call's row in `tool_calls`, which a paid generation names.
    call: i64,
    /// The revision the call was worked out against, when it names one
    /// ([`Toolbox::edit`]).
    at: Option<i64>,
    /// Tripped when whoever asked no longer wants the answer
    /// ([`Toolbox::call_cancellable`]).
    cancel: Cancel,
}

/// Why a call did not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Refusal {
    /// Refused, in words for whoever asked — MCP's `isError`.
    #[error("{0}")]
    Said(String),
    /// The project is no longer at the revision the call named, so nothing
    /// was written.
    #[error(
        "the project changed since this edit was worked out, so nothing was written — \
         redo it on what is there now"
    )]
    Moved,
}

impl From<String> for Refusal {
    fn from(said: String) -> Self {
        Self::Said(said)
    }
}

/// The project id an argument object names.
///
/// An integer, as `project_list` shows it; a string of digits is taken too,
/// because some clients send every argument as a string.
fn project_id(arguments: &Value) -> Result<i64, String> {
    let project = arguments.get("project");
    project
        .and_then(Value::as_i64)
        .or_else(|| project.and_then(Value::as_str)?.trim().parse().ok())
        .ok_or_else(|| {
            "`project` is required: the id of one of your projects, as project_list shows it"
                .to_owned()
        })
}

/// A database failure, said to the caller without its detail.
fn database(error: impl std::fmt::Display) -> String {
    eprintln!("scorsese-server: tool call: {error}");
    "the server's database failed; try again".to_owned()
}

/// The registry's tools, by name.
fn registered(name: &str) -> Option<Box<dyn Tool>> {
    scorsese_mcp::registry()
        .into_iter()
        .find(|tool| tool.name() == name)
}
