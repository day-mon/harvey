//! `harvey` — AI-native HAR (HTTP Archive) file analyzer library.
//!
//! This crate provides the core types and logic for parsing and analyzing
//! HAR 1.2 files, plus the CLI argument definitions. The `harvey` binary
//! is thin glue that just parses args and dispatches.

pub mod cli;
pub mod commands;
pub mod har;
pub mod output;
pub mod validators;
