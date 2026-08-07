//! Vertical application use cases shared by the desktop adapters.
//!
//! This crate deliberately depends on the existing persistence contracts while
//! the migration is in progress. It never depends on Tauri, HTTP, Keyring, or
//! raw SQLite connections.

pub mod agent;
pub mod coach;
pub mod error;
pub mod ports;
pub mod writing_evaluation;

pub use agent::*;
pub use coach::CoachService;
pub use error::ApplicationError;
pub use ports::*;
pub use writing_evaluation::{EvaluationBackend, StartEvaluationOutcome, WritingEvaluationService};
