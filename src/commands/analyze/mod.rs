//! `harvey analyze` — aggregate HAR statistics.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::Args;
use serde::Serialize;
use tabled::{
    builder::Builder,
    settings::{object::Rows, Color, Modify, Style},
};

use crate::cli::GlobalArgs;
use crate::har::parser;
use crate::har::stats;
use crate::output::OutputMode;

/// Analyze a HAR file and print aggregate statistics.
///
/// Best first command when inspecting a new HAR capture.
#[derive(Debug, Args)]
pub struct AnalyzeArgs {
    /// Path to the .har file.
    #[arg(value_name = "FILE", value_hint = clap::ValueHint::FilePath, value_parser = crate::validators::existing_file)]
    pub file: PathBuf,
}

/// JSON output wrapper for analyze.
#[derive(Debug, Serialize)]
struct AnalyzeOutput {
    /// Schema version identifier.
    format_version: String,
    /// The analyzed file path.
    file: String,
    /// The computed statistics.
    stats: stats::Stats,
    /// Earliest request timestamp.
    time_start: Option<String>,
    /// Latest request timestamp.
    time_end: Option<String>,
}

/// Run the analyze command.
///
/// # Errors
///
/// Returns an error if the HAR file cannot be loaded or parsed.
pub fn run(args: &AnalyzeArgs, global: &GlobalArgs) -> Result<()> {
    let mode = OutputMode::from_args(global.json);
    let har = parser::load(&args.file)?;
    let computed = stats::compute(&har.log);

    let (time_start, time_end) = time_range(&har.log.entries);

    match mode {
        OutputMode::Human => render_human(
            &args.file,
            &computed,
            time_start.as_deref(),
            time_end.as_deref(),
        )?,
        OutputMode::Json => {
            render_json(&args.file, &computed, time_start, time_end)?;
        }
    }

    Ok(())
}

/// Print a human-readable statistics table using `tabled`.
fn render_human(
    file: &Path,
    stats: &stats::Stats,
    time_start: Option<&str>,
    time_end: Option<&str>,
) -> Result<()> {
    tracing::info!("analyzing: {}", file.display());

    let mut builder = Builder::default();
    builder.push_record(["Metric", "Value"]);
    builder.push_record(["File", &file.display().to_string()]);
    builder.push_record(["Total entries", &stats.total_entries.to_string()]);
    builder.push_record(["Total bytes", &format_bytes(stats.total_bytes)]);
    builder.push_record(["Unique domains", &stats.unique_domains.to_string()]);
    builder.push_record(["Avg time", &format!("{:.1} ms", stats.avg_time_ms)]);
    builder.push_record(["P50 time", &format!("{:.0} ms", stats.p50_time_ms)]);
    builder.push_record(["P95 time", &format!("{:.0} ms", stats.p95_time_ms)]);
    builder.push_record(["P99 time", &format!("{:.0} ms", stats.p99_time_ms)]);
    builder.push_record(["Min time", &format!("{:.0} ms", stats.min_time_ms)]);
    builder.push_record(["Max time", &format!("{:.0} ms", stats.max_time_ms)]);

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::new(0..=0)).with(Color::BOLD));

    let output = table.to_string();
    let mut stdout = std::io::stdout();
    std::io::Write::write_all(&mut stdout, output.as_bytes())
        .context("failed to write table to stdout")?;
    std::io::Write::write_all(&mut stdout, b"\n")
        .context("failed to write newline")?;

    // Status distribution sub-table
    if !stats.status_distribution.is_empty() {
        tracing::info!("status distribution:");
        for (status, count) in &stats.status_distribution {
            let bar = "█".repeat(*count.min(&40));
            tracing::info!("  {:>3}  {:>4}  {}", status, count, bar);
        }
    }

    if let (Some(start), Some(end)) = (time_start, time_end) {
        tracing::info!("time range: {start} → {end}");
    }

    Ok(())
}

/// Write JSON output to stdout.
fn render_json(
    file: &Path,
    stats: &stats::Stats,
    time_start: Option<String>,
    time_end: Option<String>,
) -> Result<()> {
    let output = AnalyzeOutput {
        format_version: "1.0".into(),
        file: file.display().to_string(),
        stats: stats.clone(),
        time_start,
        time_end,
    };

    let json = serde_json::to_string_pretty(&output)
        .context("failed to serialize analyze output")?;
    let mut stdout = std::io::stdout();
    std::io::Write::write_all(&mut stdout, json.as_bytes())
        .context("failed to write JSON to stdout")?;
    std::io::Write::write_all(&mut stdout, b"\n")
        .context("failed to write newline")?;
    Ok(())
}

/// Extract the time range (earliest/latest ISO 8601 timestamps) from entries.
fn time_range(
    entries: &[crate::har::types::Entry],
) -> (Option<String>, Option<String>) {
    if entries.is_empty() {
        return (None, None);
    }

    let mut min = &entries[0].started_date_time;
    let mut max = &entries[0].started_date_time;

    for entry in &entries[1..] {
        if entry.started_date_time < *min {
            min = &entry.started_date_time;
        }
        if entry.started_date_time > *max {
            max = &entry.started_date_time;
        }
    }

    (Some(min.clone()), Some(max.clone()))
}

/// Format a byte count as a human-readable string.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
