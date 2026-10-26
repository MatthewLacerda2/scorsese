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
use scorsese_server::captures::worker::{Isolation, Worker};
use scorsese_server::generations::Keys;
use scorsese_server::operator::{self, Cli, Command};
use scorsese_server::{Config, ServerError};

#[tokio::main]
async fn main() -> ExitCode {
    scorsese_server::logs::init();
    let command = Cli::parse().command.unwrap_or(Command::Serve);
    // The capture containers' commands read no environment and no database:
    // they have neither a network nor the server's secrets.
    match command {
        Command::CaptureLauncher(args) => return capture_launcher(&args),
        Command::CaptureWorker(args) => return capture_worker(&args),
        Command::CaptureOne(args) => {
            return match scorsese_server::captures::one::run(&args) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => {
                    // lint: allow(server-print) the capture worker reads this last line as the failure
                    eprintln!("scorsese-server: {error}");
                    ExitCode::FAILURE
                }
            };
        }
        _ => {}
    }
    let here = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let outcome = match Config::from_environment(&Environment::discover(&here)) {
        Ok(config) => perform(config, command).await,
        Err(error) => Err(error.into()),
    };
    match outcome {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Serve, or carry out one operator command and print what it says.
async fn perform(config: Config, command: Command) -> Result<(), ServerError> {
    let output = match command {
        Command::Serve => {
            tracing::info!("starting on {}", config.bind);
            scorsese_server::run(config, stop_requested()).await?;
            tracing::info!("stopped");
            return Ok(());
        }
        Command::User(command) => {
            let pool = scorsese_server::open_database(&config).await?;
            operator::user(&pool, &config.files(), &Keys, command).await?
        }
        Command::Token(command) => {
            let pool = scorsese_server::open_database(&config).await?;
            operator::token(&pool, command).await?
        }
        Command::Job(command) => {
            let pool = scorsese_server::open_database(&config).await?;
            operator::job(&pool, command).await?
        }
        Command::Credit(command) => {
            let pool = scorsese_server::open_database(&config).await?;
            scorsese_server::credits::command::run(&pool, command).await?
        }
        Command::Ab(command) => {
            let pool = scorsese_server::open_database(&config).await?;
            scorsese_server::ab::report::run(&pool, command).await?
        }
        Command::CaptureLauncher(_) | Command::CaptureWorker(_) | Command::CaptureOne(_) => {
            unreachable!("handled before the environment is read")
        }
    };
    // lint: allow(server-print) an operator command's answer, on stdout for the operator
    println!("{output}");
    Ok(())
}

/// Capture pages from the spool until stopped.
fn capture_worker(args: &scorsese_server::captures::worker::Args) -> ExitCode {
    match Worker::from_args(args) {
        Ok(worker) => {
            let sandbox = match worker.isolation {
                Isolation::Process { sandbox, .. } => sandbox,
                Isolation::Container(_) => true,
            };
            tracing::info!(
                "capturing pages from {}, sandbox {}",
                args.spool.display(),
                if sandbox { "on" } else { "off" }
            );
            worker.run()
        }
        Err(error) => {
            tracing::error!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Capture pages from the spool until stopped, each in a container of its own.
fn capture_launcher(args: &scorsese_server::captures::launch::Args) -> ExitCode {
    let launch = scorsese_server::captures::launch::Launch::from_args(args);
    if let Err(error) = scorsese_server::captures::launch::ready() {
        tracing::error!("{error}");
        return ExitCode::FAILURE;
    }
    tracing::info!(
        "capturing pages from {}, a container each, sandbox on",
        args.spool.display()
    );
    Worker::launching(launch).run()
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
