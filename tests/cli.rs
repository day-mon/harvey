#![expect(
    missing_docs,
    reason = "test files do not need module-level docs"
)]

use std::path::PathBuf;

use clap::{CommandFactory, Parser};
use harvey::cli::{Cli, Command};

/// Returns the path to the example HAR fixture.
fn fixture() -> PathBuf {
    let manifest = std::env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest).join("tests/fixtures/example.har")
}

fn f() -> String {
    fixture().to_string_lossy().into_owned()
}

#[test]
fn cli_shape_is_valid() {
    Cli::command().debug_assert();
}

// ── analyze ──

#[test]
fn parses_analyze_command() {
    let cli = Cli::try_parse_from(["harvey", "analyze", &f()]).unwrap();
    assert!(matches!(cli.command, Command::Analyze(_)));
}

// ── entries ──

#[test]
fn parses_entries_defaults() {
    let cli = Cli::try_parse_from(["harvey", "entries", &f()]).unwrap();
    if let Command::Entries(args) = cli.command {
        assert_eq!(args.limit, 100);
    } else {
        panic!("expected Entries");
    }
}

#[test]
fn parses_entries_sort_asc() {
    let path = f();
    let cli =
        Cli::try_parse_from(["harvey", "entries", &path, "--sort-dir", "asc"])
            .unwrap();
    assert!(matches!(cli.command, Command::Entries(_)));
}

#[test]
fn parses_entries_invalid_sort_dir_fails() {
    let path = f();
    let result = Cli::try_parse_from([
        "harvey",
        "entries",
        &path,
        "--sort-dir",
        "sideways",
    ]);
    assert!(result.is_err());
}

// ── domains ──

#[test]
fn parses_domains_sort_by_bytes() {
    let path = f();
    let cli =
        Cli::try_parse_from(["harvey", "domains", &path, "--sort-by", "bytes"])
            .unwrap();
    assert!(matches!(cli.command, Command::Domains(_)));
}

#[test]
fn parses_domains_sort_by_avg_time() {
    let path = f();
    let cli = Cli::try_parse_from([
        "harvey",
        "domains",
        &path,
        "--sort-by",
        "avg-time",
    ])
    .unwrap();
    assert!(matches!(cli.command, Command::Domains(_)));
}

// ── inspect ──

#[test]
fn parses_inspect_with_index() {
    let path = f();
    let cli = Cli::try_parse_from(["harvey", "inspect", &path, "--entry", "5"])
        .unwrap();
    if let Command::Inspect(args) = cli.command {
        assert_eq!(args.entry, 5);
    } else {
        panic!("expected Inspect");
    }
}

// ── schema ──

#[test]
fn parses_schema_command() {
    let cli = Cli::try_parse_from(["harvey", "schema", "entries"]).unwrap();
    assert!(matches!(cli.command, Command::Schema(_)));
}

#[test]
fn parses_schema_invalid_command_fails() {
    let result = Cli::try_parse_from(["harvey", "schema", "nonexistent"]);
    assert!(result.is_err());
}

// ── global flags ──

#[test]
fn verbose_count() {
    let path = f();
    let cli =
        Cli::try_parse_from(["harvey", "-vvv", "analyze", &path]).unwrap();
    assert_eq!(cli.global.verbose, 3);
}

#[test]
fn json_flag() {
    let path = f();
    let cli =
        Cli::try_parse_from(["harvey", "--json", "analyze", &path]).unwrap();
    assert!(cli.global.json);
}

#[test]
fn no_color_flag() {
    let path = f();
    let cli = Cli::try_parse_from(["harvey", "--no-color", "analyze", &path])
        .unwrap();
    assert!(cli.global.no_color);
}

#[test]
fn verbose_and_quiet_conflict() {
    let path = f();
    let result = Cli::try_parse_from(["harvey", "-v", "-q", "analyze", &path]);
    assert!(result.is_err());
}

#[test]
fn missing_subcommand_is_error() {
    let result = Cli::try_parse_from(["harvey"]);
    assert!(result.is_err());
}
