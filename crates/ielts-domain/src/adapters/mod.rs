//! Pure legacy adapters. New writes must never emit legacy aliases.

mod evaluation_v3;
mod reading_archive;

pub use evaluation_v3::*;
pub use reading_archive::*;
