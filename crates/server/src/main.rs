//! The `scorsese-server` binary: the web API, configured by its environment.
//!
//! No arguments. Everything it needs — the database, where files go, where to
//! listen — comes from environment variables (see [`scorsese_server::config`]),
//! because that is how a container is configured and this is what runs in one.

use std::process::ExitCode;

use scorsese_providers::credentials::Environment;
use scorsese_server::Config;

#[tokio::main]
async fn main() -> ExitCode {
    let here = std::env::current_dir().unwrap_or_else(|_| ".".into());
    let outcome = match Config::from_environment(&Environment::discover(&here)) {
        Ok(config) => {
            eprintln!("scorsese-server: starting on {}", config.bind);
            scorsese_server::run(config, stop_requested()).await
        }
        Err(error) => Err(error.into()),
    };
    match outcome {
        Ok(()) => {
            eprintln!("scorsese-server: stopped");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("scorsese-server: {error}");
            ExitCode::FAILURE
        }
    }
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
