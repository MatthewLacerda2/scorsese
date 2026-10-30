//! Every integration test of `server`, in one binary (#1128).
//!
//! Each module is what used to be a test binary of its own; one binary
//! links once instead of once per file.

mod ab;
mod access;
mod accounts;
mod assistant;
mod captures;
mod client_errors;
mod common;
mod config;
mod contained;
mod credits;
mod editor;
mod events;
mod google;
mod isolation;
mod jobs;
mod library;
mod logs;
mod materialise;
mod mcp;
mod migrations;
mod operator;
mod payments;
mod project_files;
mod projects;
mod renders;
mod serve;
mod sessions;
mod stored;
mod tokens;
