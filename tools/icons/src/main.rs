//! The binary: where the icons directory is, and what to print.
//!
//! Everything it does is [`scorsese_icon_vendor::run`]; this file is the path
//! to the vendored tree and the two lines that report the answer. See the
//! crate doc for what the tool is for and why it is not a build script.

use std::path::PathBuf;
use std::process::ExitCode;

/// Where the vendored tree and the blob live, from this crate's own directory.
const ICONS: &str = "../../crates/compositor/icons";

fn main() -> ExitCode {
    let checking = std::env::args().any(|argument| argument == "--check");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(ICONS);
    match scorsese_icon_vendor::run(&root, checking) {
        Ok(message) => {
            println!("vendor-icons: {message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("vendor-icons: {message}");
            ExitCode::FAILURE
        }
    }
}
