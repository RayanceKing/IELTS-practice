//! IELTS Practice domain contracts for the Rust + Tauri rewrite.
//!
//! Phase 1 scope:
//! - domain enums and state vocabulary
//! - reading asset v2 + writing evaluation v4 schemas
//! - unified error envelope
//! - pure legacy adapters (reading archive / evaluation v3)
//! - no Tauri, no SQLite, no UI

pub mod adapters;
pub mod domain;
pub mod dto;
pub mod error;
pub mod view;

pub use domain::*;
pub use dto::*;
pub use error::*;
pub use view::*;
