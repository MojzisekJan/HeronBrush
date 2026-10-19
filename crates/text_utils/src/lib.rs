//! Shared utilities for Heron Brush.
//!
//! String comparison, glob matching and formatting helpers used by the editor.
//!
//! What remains is genuine behaviour: string comparison and glob matching with
//! case-sensitive and case-insensitive variants, and the string formatting helpers the
//! editor's parsers and UI rely on.

#![forbid(unsafe_code)]

pub mod string_compare;
pub mod string_format;
pub mod string_utils;
