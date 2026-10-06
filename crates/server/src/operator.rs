//! The operator's commands: `scorsese-server user …`, `token …`, `job …` and
//! `credit …` (the last in [`crate::credits::command`]).
//!
//! **There is no public sign-up in v1** (#533): the operator makes each
//! account by hand for somebody they know, and resets a password the same way
//! — there is no reset email. Run where the server runs, against the same
//! environment, e.g.
//!
//! ```text
//! docker compose exec scorsese-server scorsese-server user create ana@example.com
//! ```
//!
//! A password is **generated, never typed**: it is printed once, for the
//! operator to hand over, and the user changes it from the web app. A
//! password on a command line is in the shell's history and in `ps` for
//! anybody on the machine; one the operator invents is weaker than twenty
//! random characters.
//!
//! The login's brake is the operator's too: `user locks` lists who it is
//! holding back and `user unlock` lets one of them through, and a password
//! reset lifts the lock on that email ([`throttle`]).
//!
//! Each command's output is what [`user`] and [`token`] return, printed to
//! stdout by the binary — so a test reads exactly what the operator would.

use clap::Subcommand;
use sqlx::postgres::PgPool;

use crate::ServerError;
use crate::accounts::throttle::{self, Key};
use crate::accounts::{password, tokens, users};
pub use crate::credits::command::CreditCommand;
use crate::jobs::store;
use crate::storage::Storage;

/// What the binary can be asked to do.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Serve the web API. What running with no command does.
    Serve,
    /// Create, list, reset and delete accounts.
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
    /// Capture web pages for the server's renders: the `capture` container's
    /// process (#778). Needs no database and no network.
    CaptureWorker(crate::captures::worker::Args),
    /// One capture, run by `capture-worker` for each page.
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
    /// Create an account and print its generated password, once.
    Create {
        /// The email the account logs in with.
        email: String,
    },
    /// Give an account a new generated password, printed once, and log out
    /// every browser it is logged in on.
    ResetPassword {
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
    /// List every account.
    List,
    /// List the emails and addresses the login is braking: locked out now, or
    /// counting failed attempts.
    Locks,
    /// Lift the login's lock-out on an email or a client address, and forget
    /// its failed attempts — for somebody locked out who should not wait.
    Unlock {
        /// The email, or the address as `user locks` prints it.
        who: String,
    },
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
            let password = password::generate()?;
            let user = users::create(pool, &email, &password).await?;
            format!(
                "created account {} for {email}\npassword: {password}",
                user.get()
            )
        }
        UserCommand::ResetPassword { email } => {
            let password = password::generate()?;
            users::set_password(pool, &email, &password).await?;
            // Somebody handed a new password is about to type it.
            throttle::clear(pool, &Key::email(&email))
                .await
                .map_err(ServerError::Database)?;
            format!(
                "new password for {email}: {password}\nevery browser session it had is logged out"
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
                    .map(|account| format!("{}\t{}", account.id, account.email))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        UserCommand::Locks => locks(pool).await?,
        UserCommand::Unlock { who } => {
            let key = Key::parse(&who);
            if throttle::clear(pool, &key)
                .await
                .map_err(ServerError::Database)?
            {
                format!("{key} may log in again")
            } else {
                format!("{key} was not being braked")
            }
        }
    })
}

/// `user locks`: one line per counter, locked ones saying for how long.
async fn locks(pool: &PgPool) -> Result<String, ServerError> {
    let locks = throttle::locks(pool).await.map_err(ServerError::Database)?;
    if locks.is_empty() {
        return Ok("nothing is locked out".to_owned());
    }
    Ok(locks
        .iter()
        .map(|lock| {
            let state = if lock.locked_for > 0 {
                let minutes = (lock.locked_for + 59) / 60;
                format!("locked for {minutes} more min (lock-out {})", lock.strikes)
            } else {
                format!(
                    "{} of {} attempts used in the last {} min",
                    lock.attempts,
                    throttle::counter::LIMIT,
                    throttle::counter::WINDOW / 60
                )
            };
            format!("{}\t{}\t{state}", lock.key.kind(), lock.key.text())
        })
        .collect::<Vec<_>>()
        .join("\n"))
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
