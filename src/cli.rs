//! Global CLI arguments and the top-level `Cli` / `Command` parser.

use clap::{Args, Parser, Subcommand};

use crate::commands::analyze::AnalyzeArgs;
use crate::commands::capture::CaptureArgs;
use crate::commands::domains::DomainsArgs;
use crate::commands::endpoints::EndpointsArgs;
use crate::commands::entries::EntriesArgs;
use crate::commands::inspect::InspectArgs;
use crate::commands::schema::SchemaArgs;

/// Global arguments available to all subcommands.
#[derive(Debug, Clone, Args)]
pub struct GlobalArgs {
    /// Increase log verbosity (-v for debug, -vv for trace).
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress all diagnostic output (stderr). Only data is emitted.
    #[arg(short, long, global = true, action = clap::ArgAction::SetTrue, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Output as machine-readable JSON instead of human tables.
    #[arg(long, global = true, action = clap::ArgAction::SetTrue)]
    pub json: bool,

    /// Disable ANSI color codes in output.
    #[arg(long, global = true, env = "NO_COLOR", action = clap::ArgAction::SetTrue)]
    pub no_color: bool,
}

// ---------------------------------------------------------------------------
// Top-level CLI and subcommand enum
// ---------------------------------------------------------------------------

/// AI-native HAR file analyzer.
///
/// Analyze HTTP Archive files with machine-readable output designed
/// for both human developers and autonomous AI agents.
#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "AI-native HAR (HTTP Archive) file analyzer",
    long_about = "Analyze HAR files from browser DevTools with machine-readable output.\n\nDesigned for both humans and autonomous AI agents.",
    subcommand_required = true,
    arg_required_else_help = true,
    after_help = "Use `harvey help <COMMAND>` for per-command details.\nUse `harvey schema <COMMAND>` to inspect JSON output schemas."
)]
pub struct Cli {
    /// Global flags (verbose, quiet, json, no-color).
    #[command(flatten)]
    pub global: GlobalArgs,

    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Command,
}

/// Top-level subcommands for `harvey`.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Analyze a HAR file — summary statistics and distributions
    Analyze(AnalyzeArgs),

    /// List and filter individual HTTP request/response entries
    Entries(EntriesArgs),

    /// Show per-domain breakdown of all requests
    Domains(DomainsArgs),

    /// Show deduplicated API endpoint summary
    Endpoints(EndpointsArgs),

    /// Live-capture network traffic from a URL via Chrome DevTools
    Capture(CaptureArgs),

    /// Inspect a single entry in full detail — headers, cookies, timings
    Inspect(InspectArgs),

    /// Print the JSON output schema for a command
    Schema(SchemaArgs),
}
