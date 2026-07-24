//! Entry point for the `scorsese` binary. See `lib.rs` for the crate's
//! responsibility and boundary (headless — never touches a display).

fn main() -> std::process::ExitCode {
    match scorsese_cli::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            // `{error:#}` prints the whole context chain, which is what makes
            // an unattended failure diagnosable from a log alone.
            eprintln!("error: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
