//! SQLite v2 persistence for the IELTS Practice rewrite.
//!
//! Phase 3: migration chain, WAL, legacy importers, shadow-read.
//! Phase 4: unified history, settings, backup, secret refs.

pub mod backup;
pub mod history;
pub mod import;
pub mod migrate;
pub mod modes;
pub mod reading;
pub mod secrets;
pub mod settings;
pub mod shadow;
pub mod writing;
pub mod sqlite;

pub use backup::*;
pub use history::*;
pub use import::*;
pub use migrate::*;
pub use modes::*;
pub use reading::*;
pub use secrets::*;
pub use settings::*;
pub use shadow::*;
pub use writing::*;
pub use sqlite::*;
