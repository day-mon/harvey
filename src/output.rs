//! Output mode selection and rendering helpers.

/// Whether to render human-readable tables or machine-parseable JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputMode {
    /// Human-readable `tabled` output.
    Human,
    /// Machine-parseable JSON (or JSONL) output.
    Json,
}

impl OutputMode {
    /// Select the output mode based on the `--json` flag.
    #[must_use]
    pub const fn from_args(json_flag: bool) -> Self {
        if json_flag {
            Self::Json
        } else {
            Self::Human
        }
    }
}
