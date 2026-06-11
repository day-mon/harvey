---
name: clap-rust
description: Use when building or reviewing Rust command-line interfaces with clap: derive Parser/Args/Subcommand/ValueEnum, builder Command/Arg, typed value parsers, env/default handling, bool flags, repeated flags, subcommands, shell completions, help text, and CLI tests.
---

# Clap Rust

Use these conventions for Rust CLIs built with `clap-rs/clap`.

## Source Baseline

- Prefer released docs from `docs.rs/clap`, crates.io, and the matching GitHub release over older snippets.
- Current docs.rs baseline checked for this skill: `clap 4.6.1`.
- Clap builds polished command-line parsers declaratively with derive or procedurally with the builder API.
- Prefer derive for static application CLIs. Use builder API when arguments are generated dynamically, when augmenting another parser, or when the CLI shape cannot be represented cleanly with derive.

## Cargo

Use derive for most application CLIs:

```toml
[dependencies]
clap = { version = "4.6", features = ["derive", "env", "wrap_help"] }
```

Feature choices:

- Default features include `std`, `color`, `help`, `usage`, `error-context`, and `suggestions`.
- `derive` enables `#[derive(Parser)]`, `Args`, `Subcommand`, and `ValueEnum`.
- `cargo` enables macros such as `command!()` and Cargo metadata helpers.
- `env` enables reading argument values from environment variables.
- `unicode` supports Unicode in arguments and help.
- `wrap_help` wraps help text based on terminal width.
- `string` allows runtime-generated strings in builder configuration.
- Avoid `unstable-v5` unless the project explicitly opts into preview APIs.

## Derive First

Use `Parser` for the top-level CLI, `Args` for reusable argument groups, `Subcommand` for command enums, and `ValueEnum` for finite sets of allowed values.

```rust
use std::path::PathBuf;
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(flatten)]
    global: GlobalArgs,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Args)]
struct GlobalArgs {
    /// Increase logging verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Optional config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run the service
    Serve {
        #[arg(long, default_value_t = 8080)]
        port: u16,

        #[arg(long, value_enum, default_value = "fast")]
        mode: Mode,
    },
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, ValueEnum)]
enum Mode {
    Fast,
    Slow,
}

fn main() {
    let cli = Cli::parse();
    println!("{cli:#?}");
}
```

- Use doc comments for help text. Use `long_about = None` when you want concise command-level help from `about`.
- Use `Option<T>` for optional values and plain `T` for required positionals/options.
- Use `default_value_t` for typed defaults; it needs `Display` for the value type.
- Use `value_enum` for enums that should produce possible-values help and validation.
- Use `#[command(flatten)]` for shared flags, not copy-pasted fields.

## Bool and Count Flags

Be explicit with flag actions when behavior matters.

- `bool` fields with `#[arg(long)]` commonly map to set-true flags.
- Use `ArgAction::SetTrue` for explicit true flags.
- Use `ArgAction::SetFalse` for disable flags such as `--no-color`.
- Use `ArgAction::Count` for `-v`, `-vv`, `-vvv` style verbosity.
- Use `Vec<T>` or `ArgAction::Append` for repeated values.
- Use `Option<T>` when absence is semantically different from a default.

```rust
#[derive(Debug, clap::Parser)]
struct Cli {
    #[arg(long, action = clap::ArgAction::SetTrue)]
    dry_run: bool,

    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    #[arg(long = "include", value_name = "GLOB")]
    includes: Vec<String>,
}
```

## Typed Parsers and Validation

Let field types drive parsing when possible. Add `value_parser` for ranges, custom parsers, and stricter validation.

```rust
#[derive(Debug, clap::Parser)]
struct Cli {
    #[arg(long, value_parser = clap::value_parser!(u16).range(1..=65535))]
    port: u16,

    #[arg(long, value_parser = clap::builder::NonEmptyStringValueParser::new())]
    name: String,
}
```

- Prefer `PathBuf` for paths, `SocketAddr` for socket addresses, numeric types for numeric inputs, and `ValueEnum` for finite modes.
- Validate at parse time when the rule is local to one argument.
- Validate after parse when the rule depends on multiple fields, filesystem state, network state, or business logic.
- Do not parse strings manually after Clap if a typed parser can do it clearly.

## Env and Defaults

Use environment variables for deployment/config integration, but keep precedence obvious.

```rust
#[derive(Debug, clap::Parser)]
struct Cli {
    #[arg(long, env = "APP_RPC_URL")]
    rpc_url: String,

    #[arg(long, env = "APP_TIMEOUT_MS", default_value_t = 5_000)]
    timeout_ms: u64,
}
```

- Enable the `env` feature before using `#[arg(env = "...")]`.
- Document env variable names in help by using explicit names.
- Use `default_value_t` for typed defaults and `default_value` for string defaults.
- Be careful with `Option<T>` plus defaults: a default means the value is effectively always present.

## Subcommands

Use required subcommands for multi-command tools unless the bare command has useful behavior.

```rust
#[derive(Debug, clap::Parser)]
#[command(version, about, subcommand_required = true, arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}
```

- `Option<Command>` makes the subcommand optional.
- `subcommand_required = true` and `arg_required_else_help = true` make operational tools friendlier.
- Use `propagate_version = true` when subcommands should also respond to `--version`.
- Keep subcommand structs small; delegate business logic to normal functions or services after parsing.

## Builder API

Use builder API for dynamic CLIs, generated arguments, plugins, or when combining with derived args.

```rust
use clap::{Arg, ArgAction, Command, value_parser};

fn command() -> Command {
    Command::new("demo")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Demo CLI")
        .arg(
            Arg::new("port")
                .long("port")
                .value_parser(value_parser!(u16).range(1..=65535))
                .default_value("8080"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .action(ArgAction::Count),
        )
}
```

- Use stable argument IDs and retrieve with matching types: `get_one::<T>()`, `get_many::<T>()`, `get_flag()`, `get_count()`.
- In builder mode, make a `fn command() -> Command` so tests can call `debug_assert()` and `try_get_matches_from`.
- Use derive augmentation only when mixing generated and typed sections is clearer than one style alone.

## Testing

Clap reports many development errors through debug assertions. Every non-trivial CLI should test the command shape.

```rust
use clap::{CommandFactory, Parser};

#[test]
fn cli_shape_is_valid() {
    Cli::command().debug_assert();
}

#[test]
fn parses_serve_port() {
    let cli = Cli::try_parse_from(["app", "serve", "--port", "9000"]).unwrap();
    match cli.command {
        Command::Serve { port, .. } => assert_eq!(port, 9000),
    }
}
```

- Use `CommandFactory` for derive-based `debug_assert()`.
- Use `try_parse_from` or `try_get_matches_from` in tests; do not call `parse()` or `get_matches()` in tests because they exit on error.
- Add tests for required args, invalid enum values, env/default behavior, and important conflicts.
- Snapshot help output only when the project already uses snapshot tooling or CLI UX is a stable contract.

## Arg Relationships

Clap provides several mechanisms to express relationships between args. Always model these declaratively rather than validating manually after parse.

### Conflicts

```rust
/// One-to-one: --debug and --quiet are mutually exclusive.
#[arg(long, conflicts_with = "quiet")]
debug: bool,

/// One-to-many: --user-data-dir conflicts with both --profile and --connect.
#[arg(long, conflicts_with_all = ["profile", "connect"])]
user_data_dir: Option<PathBuf>,
```

- Conflict rules are two-way — declaring `A.conflicts_with(B)` is enough, no need to also do `B.conflicts_with(A)`.
- Conflicts take precedence over `required`.

### Requires

```rust
/// --all only makes sense when --filter-url is set.
#[arg(long, action = clap::ArgAction::SetTrue, requires = "filter_url")]
all: bool,

/// Filter entries by URL regex pattern.
#[arg(long, value_name = "REGEX")]
filter_url: Option<String>,
```

### Arg groups (mutual exclusivity or mutual requirement)

```rust
#[derive(clap::Args)]
#[command(group = clap::ArgGroup::new("mode").args(["connect", "url"]).required(true))]
pub struct CaptureArgs {
    #[arg(long)]
    url: Option<String>,

    #[arg(long)]
    connect: Option<String>,
}
```

Use `ArgGroup` when:
- Exactly one of N args must be present (`required(true)`).
- Multiple args are mutually exclusive (no `required`).
- You need at least one of several args (`required(true)` and `multiple(true)`).

## Shell Completions with `value_hint`

Set `value_hint` so shells autocomplete the right type of value:

```rust
/// Path to a .har file.
#[arg(value_name = "FILE", value_hint = clap::ValueHint::FilePath, value_parser = crate::validators::existing_file)]
file: PathBuf,

/// Target URL to capture traffic from.
#[arg(long, value_name = "URL", value_hint = clap::ValueHint::Url)]
url: Option<String>,

/// Path to a Chrome profile directory.
#[arg(long, value_hint = clap::ValueHint::DirPath)]
user_data_dir: Option<PathBuf>,
```

Common `ValueHint` variants: `FilePath`, `DirPath`, `Url`, `Hostname`, `Username`, `CommandName`, `CommandWithArguments`, `Other`.

Add `value_hint` to every arg that has a clear semantic type, especially `PathBuf` fields.

## Help Headings

Group related args under labeled sections in `--help` output:

```rust
#[arg(long, help_heading = "Browser config")]
no_headless: bool,

#[arg(long, help_heading = "Browser config")]
chrome: Option<PathBuf>,

#[arg(long, help_heading = "Capture behavior")]
timeout: u64,

#[arg(long, help_heading = "Capture behavior")]
watch: bool,
```

Use `help_heading` when a subcommand has >5 args spanning multiple concerns. Natural headings emerge from the groups users already think in.

## Optional-Value Flags

When a flag accepts an optional value (flag present but no value == use a default):

```rust
/// --profile            → uses "Default"
/// --profile "Work"     → uses "Work"
/// (flag absent)        → None
#[arg(long, value_name = "NAME", num_args = 0..=1, default_missing_value = "Default")]
profile: Option<String>,
```

Key combination:
- `num_args = 0..=1` — 0 or 1 values accepted
- `default_missing_value = "..."` — value when flag present but no value given
- `Option<String>` — `None` when flag absent entirely
- For bool-like semantics with `require_equals`, see `Arg::default_missing_value` docs.

## Regex and Hyphenated Values

When an arg accepts patterns that may start with `-` (regex, globs, negative numbers), the leading hyphen gets eaten as a flag:

```rust
/// --filter-url '-\d+' would fail without this.
#[arg(long, value_name = "REGEX", allow_hyphen_values = true)]
filter_url: Option<String>,
```

Use `allow_hyphen_values` on:
- Regex pattern args (`--filter-url`, `--filter-domain`)
- Glob args (`--include`, `--exclude`)
- Args that accept negative numbers without `=` syntax

Note: users can work around this with `--filter-url='-pattern'` (the `=` prevents misinterpretation). Adding `allow_hyphen_values` removes the need for the workaround.

## Help and UX

- Prefer precise `about`, field doc comments, `value_name`, and examples over long prose.
- Use consistent flag names: kebab-case on the command line, snake_case in Rust fields.
- Avoid surprising aliases unless compatibility requires them.
- Use `hide = true` only for compatibility shims or intentionally hidden advanced flags.
- Keep parse errors user-facing. After parsing, convert domain validation failures to clear messages and non-zero exit codes.

## Helper Script

Generate starter snippets without loading extra context:

```bash
bash /mnt/skills/user/clap-rust/scripts/clap-rust-bootstrap.sh derive
bash /mnt/skills/user/clap-rust/scripts/clap-rust-bootstrap.sh builder
bash /mnt/skills/user/clap-rust/scripts/clap-rust-bootstrap.sh subcommand
bash /mnt/skills/user/clap-rust/scripts/clap-rust-bootstrap.sh value-enum
bash /mnt/skills/user/clap-rust/scripts/clap-rust-bootstrap.sh test
```

The script prints JSON with a `scenario`, `cargo`, and `snippet` field.

## Review Checklist

- Is derive used for static CLIs and builder only where dynamic configuration is needed?
- Are feature flags (`derive`, `env`, `wrap_help`, `cargo`) intentional?
- Are values parsed into typed fields instead of manually parsed strings?
- Are bool, count, and repeated flags using the right `ArgAction` or field type?
- Are defaults typed and compatible with `Option<T>` semantics?
- Are subcommands required or optional intentionally?
- Does the CLI have `debug_assert()` and parse-error tests?
- Is business logic kept out of parser definitions?
- Do arg relationships use `conflicts_with` / `requires` / `ArgGroup` rather than manual validation after parse?
- Do `PathBuf` and typed-value args have `value_hint` for shell completions?
- Do commands with >5 args use `help_heading` to group related options?
- Do regex/glob/pattern args have `allow_hyphen_values` to prevent leading `-` from being eaten?
- Are optional-value flags using `num_args = 0..=1` + `default_missing_value` correctly?

## Sources

- `https://github.com/clap-rs/clap`
- `https://docs.rs/clap/latest/clap/`
- `https://docs.rs/clap/latest/clap/_derive/index.html`
- `https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html`
- `https://docs.rs/clap/latest/clap/_tutorial/index.html`
- `https://docs.rs/clap/latest/clap/_features/index.html`
- `https://docs.rs/clap/latest/clap/struct.Command.html`
- `https://docs.rs/clap/latest/clap/struct.Arg.html`
- `https://docs.rs/clap/latest/clap/enum.ArgAction.html`
- `https://docs.rs/clap/latest/clap/trait.ValueEnum.html`
