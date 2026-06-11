//! `harvey schema` — introspect JSON output schemas.

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};

use crate::cli::GlobalArgs;

/// Print the JSON output schema for a command.
///
/// AI agents use this to understand output structure before calling
/// other commands with `--json`.
#[derive(Debug, Clone, Copy, Args)]
pub struct SchemaArgs {
    /// Which command's schema to display.
    #[arg(value_enum, value_name = "COMMAND")]
    pub command: SchemaCommand,
}

/// Available schemas to display.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum SchemaCommand {
    /// Schema for `harvey analyze --json` output.
    Analyze,
    /// Schema for `harvey entries --json` output (JSONL format).
    Entries,
    /// Schema for `harvey domains --json` output.
    Domains,
    /// Schema for `harvey inspect --json` output.
    Inspect,
}

/// Run the schema command.
///
/// # Errors
///
/// Returns an error if the schema file is missing or invalid.
pub fn run(args: &SchemaArgs, _global: &GlobalArgs) -> Result<()> {
    let schema = get_schema(args.command)?;
    // Schema output is always plain data — write directly to stdout.
    // No Diagnostics emitted (schemas are consumed by agents).
    let mut stdout = std::io::stdout();
    std::io::Write::write_all(&mut stdout, schema.as_bytes())
        .context("failed to write schema to stdout")?;
    std::io::Write::write_all(&mut stdout, b"\n")
        .context("failed to write newline to stdout")?;
    Ok(())
}

/// Return the embedded JSON schema for the given command.
fn get_schema(command: SchemaCommand) -> Result<String> {
    let schema_str = match command {
        SchemaCommand::Analyze => {
            include_str!("../../schemas/analyze-output.json")
        }
        SchemaCommand::Entries => {
            include_str!("../../schemas/entries-output.json")
        }
        SchemaCommand::Domains => {
            include_str!("../../schemas/domains-output.json")
        }
        SchemaCommand::Inspect => {
            include_str!("../../schemas/inspect-output.json")
        }
    };
    // Validate that it's actually valid JSON.
    let _parsed: serde_json::Value = serde_json::from_str(schema_str)
        .context("embedded schema is not valid JSON")?;
    Ok(schema_str.to_owned())
}
