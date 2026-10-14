//! The operator's commands: `scorsese-server user …`, `token …`, `job …` and
//! `credit …` (the last in [`crate::credits::command`]).
//!
//! **People sign themselves up, with Google** (#952), so accounts mostly make
//! themselves. What is left for the operator, run where the server runs,
//! against the same environment, e.g.
//!
//! ```text
//! docker compose exec server scorsese-server user create ana@example.com
//! ```
//!
//! is setting an account up before its owner arrives (`user create`: their
//! first Google sign-in under that address links it), linking one by hand
//! when the addresses differ (`user link`, with the Google id the server's
//! log or the person can give), ending every browser session an account has
//! (`user logout`), and deleting one. There are no passwords to reset.
//!
//! `user session` opens a browser session and prints its cookie — for a
//! development stack with no Google client, and for the web app's end-to-end
//! tests, which sign in without Google the same way.
//!
//! Each command's output is what [`user`] and [`token`] return, printed to
//! stdout by the binary — so a test reads exactly what the operator would.

use clap::Subcommand;
use sqlx::postgres::PgPool;

use crate::ServerError;
use crate::accounts::{sessions, tokens, users};
pub use crate::credits::command::CreditCommand;
use crate::http::auth::SESSION_COOKIE;
use crate::jobs::store;
use crate::storage::Storage;

/// What the binary can be asked to do.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Serve the web API. What running with no command does.
    Serve,
    /// Create, link, list and delete accounts.
    #[command(subcommand)]
    User(UserCommand),
    /// Issue an API token on a user's behalf.
    #[command(subcommand)]
    Token(TokenCommand),
    /// See what the job queue has been through.
    #[command(subcommand)]
    Job(JobCommand),
    /// Record top-ups and refunds, read a balance.
    #[command(subcommand)]
    Credit(CreditCommand),
    /// Capture web pages for the server's renders, each capture a container
    /// of its own: the `capture-launcher` container's process (#852). Needs
    /// the Docker socket, and no database and no network.
    CaptureLauncher(crate::captures::launch::Args),
    /// Capture web pages for the server's renders without containers, each a
    /// child process: for a machine without Docker (#778).
    CaptureWorker(crate::captures::worker::Args),
    /// One capture, run by `capture-launcher` or `capture-worker` per page.
    #[command(hide = true)]
    CaptureOne(crate::captures::one::Args),
}

/// `scorsese-server job …`
#[derive(Debug, Subcommand)]
pub enum JobCommand {
    /// List the jobs a crash or a restart cut off, newest first, with whose
    /// they are and where each is now — who a power cut affected.
    Interrupted,
}

/// `scorsese-server user …`
#[derive(Debug, Subcommand)]
pub enum UserCommand {
    /// Create an account for an email, linked to its owner's Google account
    /// when they first sign in with that address.
    Create {
        /// The email the account is for.
        email: String,
    },
    /// Link an account to a Google account by Google's id for it (`sub`), for
    /// somebody whose Google email differs from the account's.
    Link {
        /// The account's email.
        email: String,
        /// Google's id for the person's account, digits like
        /// 110169484474386276334.
        google_sub: String,
    },
    /// End every browser session an account has. Its API tokens stay.
    Logout {
        /// The account's email.
        email: String,
    },
    /// Open a browser session for an account and print the cookie, once —
    /// for a stack with no Google client, or a test.
    Session {
        /// The account's email.
        email: String,
    },
    /// Delete an account with all its data and files. Cannot be undone.
    Delete {
        /// The account's email.
        email: String,
        /// Confirm: without it, nothing is deleted.
        #[arg(long)]
        yes: bool,
    },
    /// List every account, with the Google account each signs in as.
    List,
}

/// `scorsese-server token …`
#[derive(Debug, Subcommand)]
pub enum TokenCommand {
    /// Issue an API token for a user and print it, once — for a script or an
    /// MCP client set up on their behalf.
    Create {
        /// The account's email.
        email: String,
        /// What to call the token, to tell it apart when revoking it.
        name: String,
    },
}

/// Carry out a `user` command. Returns what to print.
pub async fn user(
    pool: &PgPool,
    storage: &Storage,
    command: UserCommand,
) -> Result<String, ServerError> {
    Ok(match command {
        UserCommand::Create { email } => {
            let user = users::create(pool, &email).await?;
            format!(
                "created account {} for {email}; it is linked when they sign in with Google",
                user.get()
            )
        }
        UserCommand::Link { email, google_sub } => {
            let user = users::link(pool, &email, &google_sub).await?;
            format!(
                "account {} ({email}) now signs in as Google account {}",
                user.get(),
                google_sub.trim()
            )
        }
        UserCommand::Logout { email } => {
            let user = users::find(pool, &email).await?;
            let ended = sessions::close_all(pool, user).await?;
            format!("ended {ended} browser session(s) for {email}")
        }
        UserCommand::Session { email } => {
            let user = users::find(pool, &email).await?;
            let cookie = sessions::open(pool, user).await?;
            format!(
                "session for {email}, valid {} days, shown once:\n{SESSION_COOKIE}={cookie}",
                sessions::LIFETIME_DAYS
            )
        }
        UserCommand::Delete { email, yes: false } => {
            return Err(ServerError::Unconfirmed(format!(
                "this deletes {email}'s account, every project and file it has, for good; \
                 run again with --yes"
            )));
        }
        UserCommand::Delete { email, yes: true } => {
            let user = users::delete(pool, storage, &email).await?;
            format!("deleted account {} ({email}) and its files", user.get())
        }
        UserCommand::List => {
            let accounts = users::list(pool).await?;
            if accounts.is_empty() {
                "no accounts yet".to_owned()
            } else {
                accounts
                    .iter()
                    .map(|listed| {
                        let google = listed.google_sub.as_deref().unwrap_or("not signed in yet");
                        let name = listed.account.name.as_deref().unwrap_or("-");
                        let account = &listed.account;
                        format!("{}\t{}\t{name}\t{google}", account.id, account.email)
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
    })
}

/// Carry out a `token` command. Returns what to print.
pub async fn token(pool: &PgPool, command: TokenCommand) -> Result<String, ServerError> {
    let TokenCommand::Create { email, name } = command;
    let user = users::find(pool, &email).await?;
    let issued = tokens::issue(pool, user, &name).await?;
    Ok(format!(
        "token {} for {email}, shown once:\n{}",
        issued.id, issued.token
    ))
}

/// Carry out a `job` command. Returns what to print.
pub async fn job(pool: &PgPool, command: JobCommand) -> Result<String, ServerError> {
    let JobCommand::Interrupted = command;
    let jobs = store::interrupted(pool)
        .await
        .map_err(ServerError::Database)?;
    if jobs.is_empty() {
        return Ok("no job has been interrupted".to_owned());
    }
    Ok(jobs
        .iter()
        .map(|job| {
            format!(
                "{}\t{}\t{}\t{}\tinterrupted {}",
                job.id, job.email, job.kind, job.state, job.interrupted_at
            )
        })
        .collect::<Vec<_>>()
        .join("\n"))
}
