//! `harvey` — AI-native HAR (HTTP Archive) file analyzer.
//!
//! Use `harvey help` for command overview or `harvey help <COMMAND>`
//! for per-command details.

use std::process::ExitCode;

use clap::Parser;
use harvey::cli::Command;
use harvey::cli::{Cli, GlobalArgs};
use harvey::commands;
use harvey::har::parser::ParseError;

/// Exit codes for AI agent branching.
const EXIT_FILE_NOT_FOUND: u8 = 2;
const EXIT_INVALID_HAR: u8 = 3;

/// Entry point. Sets up tracing, dispatches subcommands, and maps
/// errors to exit codes for AI agent consumption.
fn main() -> ExitCode {
    let cli = Cli::parse();

    setup_tracing(&cli.global);

    let result = match cli.command {
        Command::Analyze(args) => commands::analyze::run(&args, &cli.global),
        Command::Entries(args) => commands::entries::run(&args, &cli.global),
        Command::Domains(args) => commands::domains::run(&args, &cli.global),
        Command::Endpoints(args) => {
            commands::endpoints::run(&args, &cli.global)
        }
        Command::Capture(args) => {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("failed to create async runtime: {e}");
                    return ExitCode::from(1);
                }
            };
            rt.block_on(commands::capture::run(&args, &cli.global))
        }
        Command::Inspect(args) => commands::inspect::run(&args, &cli.global),
        Command::Schema(args) => commands::schema::run(&args, &cli.global),
    };

    match result {
        Ok(()) => ExitCode::from(0),
        Err(err) => map_error(&err),
    }
}

/// Configure `tracing-subscriber` based on global flags.
fn setup_tracing(global: &GlobalArgs) {
    let log_level = if global.quiet {
        "harvey=error".to_owned()
    } else {
        match global.verbose {
            0 => std::env::var("RUST_LOG")
                .ok()
                .unwrap_or_else(|| "harvey=info".to_owned()),
            1 => "harvey=debug".to_owned(),
            _ => "harvey=trace".to_owned(),
        }
    };

    let use_ansi = !global.no_color && std::env::var("NO_COLOR").is_err();

    let builder = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_new(&log_level).unwrap_or_else(
                |_| tracing_subscriber::EnvFilter::new("harvey=info"),
            ),
        )
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .with_ansi(use_ansi)
        .without_time();

    if global.quiet {
        builder.with_writer(std::io::stderr).init();
    } else {
        builder.init();
    }
}

/// Map an error to the appropriate exit code.
///
/// Parses the error chain for known [`ParseError`] variants to set
/// specific exit codes that AI agents can branch on.
fn map_error(err: &anyhow::Error) -> ExitCode {
    // Check for parse errors to set specific exit codes.
    if let Some(parse_err) = err.downcast_ref::<ParseError>() {
        match parse_err {
            ParseError::FileNotFound(path) => {
                tracing::error!("file not found: {path}");
                return ExitCode::from(EXIT_FILE_NOT_FOUND);
            }
            ParseError::InvalidJson { path, source } => {
                tracing::error!("invalid JSON in {path}: {source}");
                return ExitCode::from(EXIT_INVALID_HAR);
            }
            ParseError::MissingLog { path } => {
                tracing::error!(
                    "invalid HAR structure in {path}: missing 'log' field"
                );
                return ExitCode::from(EXIT_INVALID_HAR);
            }
            ParseError::NoEntries { path } => {
                tracing::warn!("HAR file {path} contains no entries");
                return ExitCode::from(0);
            }
            ParseError::Io { path, source } => {
                tracing::error!("I/O error reading {path}: {source}");
                return ExitCode::from(1);
            }
        }
    }

    // Check for "no results" sentinel from entries command.
    if err.to_string().contains("NO_RESULTS") {
        return ExitCode::from(4);
    }

    // All other errors → exit code 1.
    tracing::error!("{err:#}");
    ExitCode::from(1)
}
