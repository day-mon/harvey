//! Rendering helpers.

use anyhow::{Context, Result};
use tabled::{
    builder::Builder,
    settings::{object::Rows, Color, Modify, Style},
};

use super::types::CaptureEntry;

pub(super) fn render_human(entries: &[CaptureEntry]) -> Result<()> {
    let mut builder = Builder::default();
    builder.push_record(["Method", "Status", "URL", "Size", "Content-Type"]);

    for e in entries {
        builder.push_record([
            e.method.clone(),
            e.status.to_string(),
            truncate_url(&e.url, 80),
            format_size(e.size),
            e.mime_type.clone(),
        ]);
    }

    let mut table = builder.build();
    table
        .with(Style::rounded())
        .with(Modify::new(Rows::first()).with(Color::BOLD));

    let mut stdout = std::io::stdout();
    std::io::Write::write_all(&mut stdout, table.to_string().as_bytes())
        .context("failed to write table to stdout")?;
    std::io::Write::write_all(&mut stdout, b"\n")
        .context("failed to write newline")?;
    Ok(())
}

pub(super) fn render_jsonl(entries: &[CaptureEntry]) -> Result<()> {
    let mut stdout = std::io::BufWriter::new(std::io::stdout());
    for entry in entries {
        let line = serde_json::to_string(entry)
            .context("failed to serialize capture entry")?;
        std::io::Write::write_all(&mut stdout, line.as_bytes())
            .context("failed to write entry to stdout")?;
        std::io::Write::write_all(&mut stdout, b"\n")
            .context("failed to write newline")?;
    }
    Ok(())
}

pub(super) fn truncate_url(url: &str, max_len: usize) -> String {
    if url.len() <= max_len {
        url.to_owned()
    } else {
        format!("{}…", &url[..url.floor_char_boundary(max_len - 1)])
    }
}

pub(super) fn format_size(bytes: f64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    if bytes >= MB {
        format!("{:.1} MB", bytes / MB)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes / KB)
    } else {
        format!("{bytes:.0} B")
    }
}
