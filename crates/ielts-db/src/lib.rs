//! SQLite v2 persistence for the IELTS Practice rewrite.
//!
//! Phase 3: single migration chain, WAL, legacy importers, shadow-read diffs.
//! Does not replace Electron runtime writes yet.

pub mod import;
pub mod migrate;
pub mod shadow;
pub mod sqlite;

pub use import::*;
pub use migrate::*;
pub use shadow::*;
pub use sqlite::*;
