//! The architecture's boundaries (#1224): each rule flags a case that breaks
//! it, passes one that keeps it, and takes an inline allow with a reason.

mod edges;
mod fences;
mod headless;
mod manifest;
