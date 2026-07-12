//! IELTS Practice domain contracts for the Rust + Tauri rewrite.
//!
//! Canonical enums, DTOs, error envelope, view models.
//! Legacy converters live in `ielts-db::import::convert` (optional one-shot import only).

pub mod domain;
pub mod dto;
pub mod error;
pub mod view;

pub use domain::*;
pub use dto::*;
pub use error::*;
pub use view::*;
