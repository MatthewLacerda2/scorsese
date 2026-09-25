//! The operator's commands: `scorsese-server user …` and `token …`.
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
//! Each command's output is what [`user`] and [`token`] return, printed to
//! stdout by the binary — so a test reads exactly what the operator would.

use std::path::Path;

use clap::Subcommand;
use sqlx::postgres::PgPool;

use crate::ServerError;
use crate::accounts::{password, tokens, users};

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
    storage: &Path,
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
