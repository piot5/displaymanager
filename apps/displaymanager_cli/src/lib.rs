//! Library root for displaymanager_cli.
//!
//! Re-exports the sub-modules so integration tests (in `tests/`) and
//! downstream crates can access them via `displaymanager_cli::set` etc.

pub mod cli;
pub mod ddc;
pub mod info;
pub mod set;
