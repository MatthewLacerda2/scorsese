//! Every integration test of `golden`, in one binary (#1128).
//!
//! Each module is what used to be a test binary of its own; one binary
//! links once instead of once per file.

mod comparing;
mod failing;
mod goldens;
mod provenance;
