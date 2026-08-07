//! SQLite v2 persistence for the IELTS Practice rewrite.
//!
//! Product hot path: history, settings, writing, reading, modes, enrichment, attempts.
//! Cold path only: `import` (optional legacy one-shot migration).

pub mod agent;
pub mod annotations;
pub mod attempts;
pub mod backup;
pub mod coach;
pub mod dictionary;
pub mod history;
pub mod import;
pub mod migrate;
pub mod modes;
pub mod perf;
pub mod reading;
pub mod secrets;
pub mod settings;
pub mod shadow;
pub mod sqlite;
pub mod vocab;
pub mod writing;

pub use agent::*;
pub use annotations::*;
pub use attempts::{count_attempts, ensure_asset_stub, upsert_attempt};
pub use backup::*;
pub use coach::*;
pub use dictionary::*;
pub use history::*;
pub use import::{
    find_legacy_db_candidates, import_browser_export_file, import_browser_export_value,
    import_reading_archive_file, list_history_view_models, migrate_legacy_sqlite_to_v2,
    scan_legacy_sqlite, LegacyDbScan, LegacyMigrationReport,
};
pub use migrate::*;
pub use modes::*;
pub use perf::*;
pub use reading::*;
pub use secrets::*;
pub use settings::*;
pub use shadow::*;
pub use sqlite::*;
pub use vocab::*;
pub use writing::*;
