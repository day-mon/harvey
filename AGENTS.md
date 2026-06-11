# AGENTS.md — harvey

## Quick commands

```bash
cargo test                          # 61 tests across lib + cli + integration
cargo test --test cli               # just CLI parse tests
cargo test --test integration       # just HAR library integration tests
cargo build --release
```

No special setup, services, or fixtures needed. Everything is self-contained.

## Architecture

```
src/
  lib.rs             ← pub mod cli/commands/har/output/validators
  cli.rs             ← GlobalArgs + Cli + Command (top-level parser)
  validators.rs      ← shared clap value_parser fns (existing_file, etc.)
  output.rs          ← OutputMode enum (Human | Json)
  commands/          ← one file per subcommand, each has pub fn run()
  har/               ← HAR domain: parsing, filtering, and statistics
tests/
  cli.rs             ← integration tests for CLI parsing
  integration.rs     ← HAR parser/filter/stats tests
  fixtures/example.har  ← 8-entry sample HAR file
```

**Library-first structure**: Everything lives in the library crate. `src/main.rs` is ~120 lines of thin glue — imports, parse, dispatch match, setup_tracing, map_error. Do not put business logic or CLI definitions in `main.rs`.

**Domain modules** sit at the top level under `src/` alongside `commands/` and `cli.rs`. `har/` is the HAR domain. New domains (e.g. exporting, replay, diff) would get their own top-level directory: `src/export/`, `src/diff/`, etc.

## Lint rules (very strict — enforced at compile time)

The `Cargo.toml` `[lints]` section is aggressive. Common gotchas:

- **Every public struct/enum needs `#[derive(Debug)]`** (or manual impl). `missing_debug_implementations` is deny.
- **Every public item needs a doc comment** (`missing_docs` deny). Add `/// ...` on all `pub struct`, `pub fn`, `pub enum`.
- **`Copy`able types should derive `Copy`** (`missing_copy_implementations` deny).
- **No `unwrap()`, `expect()`, `panic!()`, `todo!()`, `unimplemented!()`** — all deny/forbid.
- **No `println!()` or `eprintln!()`** — use `tracing::*` for diagnostics, `std::io::stdout().write_all()` for data.
- **No `#[allow(...)]`** — fix the warning instead.
- `unsafe_code` is `forbid`.
- All clippy groups are `deny` (all, pedantic, nursery, cargo).

New code that doesn't derive `Debug`, `Copy`, or misses docs will fail to compile.

## Clap conventions

Established during a clap-rust skill review:

- **Bool flags**: always `#[arg(long, action = clap::ArgAction::SetTrue)]` — not implicit.
- **Typed defaults**: use `default_value_t = 42`, never `default_value = "42"`.
- **Enums**: `#[derive(ValueEnum)]` + `#[arg(value_enum, default_value_t = ...)]`.
- **Enum variant names**: use `#[value(name = "kebab-case")]` not `#[clap(name = ...)]`.
- **Subcommand UX**: `subcommand_required = true, arg_required_else_help = true` on `Cli`.
- **Features**: `clap = { version = "4.6", features = ["derive", "env", "wrap_help"] }`.
- **Validators** live in `src/validators.rs`, referenced via `crate::validators::*` in command files.
- Every non-trivial CLI needs `debug_assert()` + parse tests. See `tests/cli.rs` for patterns.

## AI-native design constraints

These are fixed rules from `CONSTITUTION.md`. Do not break them:

- **Every command supports `--json`** for machine-readable output.
- **Stdout = data, stderr = diagnostics** — clean pipe separation.
- **Output is deterministic**: same input → same output.
- **Never interactive**: no prompts, confirmations, or stdin reads.
- **Exit codes are explicit**:
  - 0 = success
  - 1 = general error
  - 2 = file not found
  - 3 = invalid HAR structure
  - 4 = no results matched (entries/capture command)
- Commands signal "no results" by returning `Err(anyhow::anyhow!("NO_RESULTS"))`. The sentinel string is checked in `map_error()`.
- JSON output uses `serde_json::to_string()` and `std::io::stdout().write_all()` — never `println!()`.
- Computed fields added by harvey go under `_computed` namespace in JSON output.
- Output schemas are versioned (`format_version`) and introspectable via `harvey schema <COMMAND>`.

## Testing patterns

- **Fixture**: `tests/fixtures/example.har` (8 entries, 4 domains, mixed status codes).
- **CLI parse tests**: use `Cli::try_parse_from([...])`, never `parse()` (which `exit()`s on failure). Use the fixture path for tests that need a real file, since `existing_file` validator rejects fake paths.
- **Integration tests**: add to `tests/integration.rs` or `tests/cli.rs`.
- **Unit tests**: `#[cfg(test)] mod tests { ... }` inside the module they test (see `src/har/parser.rs`, `src/har/stats.rs`, `src/har/filter.rs`).

## Key dependencies

| Crate | Role |
|-------|------|
| `clap 4.6` | CLI parsing (derive mode) |
| `serde` / `serde_json` | Serialization and HAR parsing |
| `tabled 0.18` | Human-readable table output |
| `tracing` + `tracing-subscriber` | Structured logging |
| `thiserror` | Library error types |
| `anyhow` | Application error handling |
| `tokio` | Async runtime (capture command only) |
| `chromiumoxide` | Chrome DevTools Protocol (capture command) |

The capture command is the only async path. `main()` creates a one-off tokio runtime only for that subcommand.

## Output modes

The `OutputMode` enum in `src/output.rs` is the central switch:
- `Human` → `tabled`-based tables to stdout
- `Json` → `serde_json` to stdout

Every command's `run()` starts with `let mode = OutputMode::from_args(global.json);` and branches on it for rendering. Follow this pattern for new commands.
