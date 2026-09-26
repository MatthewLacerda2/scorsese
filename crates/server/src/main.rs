//! The `scorsese-server` binary: the web API, configured by its environment.
//!
//! With no command it serves. Everything it needs — the database, where files
//! go, where to listen — comes from environment variables (see
//! [`scorsese_server::config`]), because that is how a container is configured
//! and this is what runs in one. The commands beside `serve` are the
//! operator's ([`scorsese_server::operator`]): they read the same environment
//! and act on the same database.

use std::process::ExitCode;

use clap::Parser;
use scorsese_providers::credentials::Environment;
use scorsese_server::operator::{self, Command};
use scorsese_server::{Config, ServerError};

/// The scorsese web API, and the operator's commands for its accounts.
///
/// Configured by the environment: DATABASE_URL, SCORSESE_STORAGE and
/// optionally SCORSESE_BIND (see .env.example).
#[derive(Debug, Parser)]
#[command(name = "scorsese-server", version)]
struct Cli {
    /// What to do. Serves when omitted.
    #[command(subcommand)]
    command: Option<Command>,
}

#[tokio::main]
async fn main() -> ExitCode {
    let command = Cli::parse().command.unwrap_or(Command::Serve);
    let here = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let outcome = match Config::from_environment(&Environment::discover(&here)) {
        Ok(config) => perform(config, command).await,
        Err(error) => Err(error.into()),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("scorsese-server: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Serve, or carry out one operator command and print what it says.
async fn perform(config: Config, command: Command) -> Result<(), ServerError> {
    let output = match command {
        Command::Serve => {
            eprintln!("scorsese-server: starting on {}", config.bind);
            scorsese_server::run(config, stop_requested()).await?;
            eprintln!("scorsese-server: stopped");
            return Ok(());
        }
        Command::User(command) => {
            let pool = scorsese_server::open_database(&config).await?;
            operator::user(&pool, &config.storage, command).await?
        }
        Command::Token(command) => {
            let pool = scorsese_server::open_database(&config).await?;
            operator::token(&pool, command).await?
        }
    };
    println!("{output}");
    Ok(())
}

/// Resolves when the process is asked to stop: Ctrl-C at a terminal, or the
/// SIGTERM `docker stop` sends. Either one starts a graceful shutdown.
async fn stop_requested() {
    let interrupt = async {
        // A handler that cannot be installed is a server that can only be
        // killed, not stopped; waiting forever on this branch leaves the
        // other signal to do the job.
        if tokio::signal::ctrl_c().await.is_err() {
            std::future::pending::<()>().await;
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{SignalKind, signal};
        match signal(SignalKind::terminate()) {
            Ok(mut stream) => {
                stream.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = interrupt => {}
        () = terminate => {}
    }
}
