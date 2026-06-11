---
name: harvey
description: HAR (HTTP Archive) file analysis CLI. Use when analyzing .har files
from browser DevTools — finding slow requests, identifying HTTP errors,
auditing third-party domains, inspecting API traffic, or understanding
network performance patterns. AI-native: every command supports --json
with documented exit codes for agent branching.
---

# Harvey — HAR File Analyzer

Analyze HAR (HTTP Archive) files from the command line. Built for both
human developers and AI agents. All output is available in machine-readable
JSON with explicit exit codes.

## Quick Reference

```
harvey analyze <FILE>            # Summary statistics, distributions, percentiles
harvey entries <FILE> [flags]    # List and filter individual requests (JSONL)
harvey inspect <FILE> --entry N  # Full detail: headers, cookies, timings, body
harvey domains <FILE>            # Per-domain breakdown: request count, bytes, times
harvey schema <COMMAND>          # Print the JSON output schema for any command
```

## When to Use

- A user provides a `.har` file and asks you to analyze it
- Debugging slow page loads — find the slowest requests and worst domains
- Auditing third-party domains on a page — `domains` shows all hosts contacted
- Finding HTTP errors (4xx, 5xx) in a capture — `entries --filter-status 500`
- Checking API call patterns — filter by method, URL regex, or content type
- Comparing before/after performance between two HAR captures

## Global Flags

| Flag | Effect |
|------|--------|
| `--json` | Machine-readable JSON/JSONL output to stdout. Always use this when consuming output programmatically. |
| `--quiet` / `-q` | Suppress all diagnostic output (stderr). Only data goes to stdout. |
| `--verbose` / `-v` | Increase log verbosity. `-v` = debug, `-vv` = trace. |
| `--no-color` | Disable ANSI color codes in terminal output. |

## Exit Codes (Agent Branching)

Harvey uses specific exit codes so you can branch without parsing error text:

| Code | Meaning | What to do |
|------|---------|------------|
| 0 | Success | Process the output (stdout has data). |
| 1 | General error | Something unexpected failed. Report the error to the user. |
| 2 | File not found | The `.har` file path is wrong. Ask the user for the correct path. |
| 3 | Invalid HAR | The file exists but isn't valid HAR JSON. Let the user know. |
| 4 | No results | Filters matched zero entries. Not an error — just convey "nothing matched." |

### Bash branching example

```bash
harvey entries capture.har --filter-status 500 --json --quiet
case $? in
  0) echo "Found 500s — processing...";;
  4) echo "No 500 errors in capture";;
  2) echo "File not found — check path";;
esac
```

## Subcommand Reference

### `harvey analyze <FILE>`

One-shot summary of the entire HAR capture. Run this first when exploring
a new file.

```
harvey analyze capture.har
harvey analyze capture.har --json
```

**Human output:** Table with total entries, total bytes, time percentiles
(P50/P95/P99/min/max), status code distribution (bar chart), and time range.

**JSON output:** Single object with `format_version`, `file`, `stats` (all
aggregate metrics), `time_start`, `time_end`.

```json
{
  "format_version": "1.0",
  "file": "capture.har",
  "stats": {
    "total_entries": 142,
    "total_bytes": 2837462,
    "avg_time_ms": 187.3,
    "p50_time_ms": 95,
    "p95_time_ms": 1200,
    "p99_time_ms": 3400,
    "status_distribution": {"200": 98, "404": 5, "500": 2},
    "content_type_distribution": {"application/json": 45, "text/html": 12},
    "method_distribution": {"GET": 110, "POST": 25},
    "unique_domains": 4
  },
  "time_start": "2024-01-01T00:00:00.000Z",
  "time_end": "2024-01-01T00:00:05.200Z"
}
```

### `harvey entries <FILE> [flags]`

List and filter individual HTTP request/response entries. The workhorse
command for drilling into specific requests.

```
harvey entries capture.har --filter-status 500
harvey entries capture.har --filter-url "/api/" --filter-method POST --limit 10
harvey entries capture.har --filter-mime "application/json" --sort-by time --json
```

| Flag | Type | Effect |
|------|------|--------|
| `--filter-url <REGEX>` | regex | Match against full request URL |
| `--filter-status <CODE>` | u16 | Exact HTTP status code |
| `--filter-method <METHOD>` | string | GET, POST, PUT, DELETE... |
| `--filter-mime <TYPE>` | string | e.g. `application/json`, `text/html` |
| `--filter-domain <DOMAIN>` | string | Exact host match |
| `--sort-by <FIELD>` | enum | `time`, `size`, `status`, `url` |
| `--sort-dir <DIR>` | enum | `asc` or `desc` (default: desc) |
| `--limit <N>` | usize | Max entries to return (default: 100) |
| `--json` | flag | JSONL output — one JSON object per line |
| `--quiet` | flag | Suppress stderr diagnostics |

**JSONL output:** Each line is a self-contained JSON object with the full
HAR entry plus computed fields under `_computed`:

```jsonl
{"startedDateTime":"...","time":234,"request":{...},"response":{...},"_computed":{"total_bytes":4567,"domain":"api.example.com","content_type":"application/json"}}
{"startedDateTime":"...","time":89,"request":{...},"response":{...},"_computed":{"total_bytes":1234,"domain":"cdn.example.com","content_type":"image/png"}}
```

**`_computed` fields:**
- `total_bytes` — sum of all request + response header and body sizes
- `domain` — host extracted from request URL
- `content_type` — shortcut to `response.content.mimeType`

**Human output:** Table with columns: `#`, `Method`, `Status`, `Domain`,
`Path`, `Time`, `Size`, `Content-Type`.

### `harvey inspect <FILE> --entry <INDEX>`

Show a single HAR entry in full detail — all request and response headers,
cookies, query string, post body, timings (with bottleneck marker), and a
body preview.

```
harvey inspect capture.har --entry 3
harvey inspect capture.har --entry 1 --json
```

| Flag | Effect |
|------|--------|
| `--entry <INDEX>` | 1-based entry index (default: 1) |
| `--json` | Full structured JSON (single object, not JSONL) |
| `--quiet` | Suppress stderr diagnostics |

**Human output:** Multi-section view with request (method, URL, headers,
cookies, query, body), response (status, headers, cookies, content, body
preview), and timings (each phase with the bottleneck highlighted).

**JSON output:** Single object with full `request`, `response`, `timings`,
`cache` objects plus `_computed` (total_bytes, domain, content_type,
bottleneck).

**Use with entries:** Scan with `harvey entries`, then drill into a specific
entry with `harvey inspect --entry N` where N matches the `#` column (when
no filters are applied).

### `harvey domains <FILE>`

Per-domain breakdown of all requests. Use to audit third-party services.

```
harvey domains capture.har
harvey domains capture.har --sort-by bytes --json
```

| Flag | Effect |
|------|--------|
| `--sort-by <METRIC>` | `requests`, `bytes`, or `avg-time` |
| `--json` | Structured JSON |
| `--quiet` | Suppress stderr |

**JSON output:**

```json
{
  "format_version": "1.0",
  "domains": [
    {
      "domain": "api.example.com",
      "request_count": 45,
      "total_bytes": 234000,
      "avg_time_ms": 120.5,
      "status_summary": {"200": 40, "401": 3, "500": 2}
    }
  ]
}
```

### `harvey schema <COMMAND>`

Print the JSON output schema for a command. Use this to understand the
output shape before calling a command with `--json`.

```
harvey schema analyze   # Schema for analyze --json output
harvey schema entries   # Schema for entries --json output (JSONL)
harvey schema domains   # Schema for domains --json output
```

Output is JSON Schema (draft 2020-12). Always valid JSON on stdout.

## Common Recipes

### Find all failed requests (4xx, 5xx)

```bash
# All 500s
harvey entries capture.har --filter-status 500 --json --quiet

# All 4xx errors
harvey entries capture.har --filter-url "" --json --quiet | jq 'select(.response.status >= 400 and .response.status < 500)'

# All errors (4xx + 5xx) — use the stats overview first
harvey analyze capture.har --json --quiet | jq '.stats.status_distribution'
```

### Identify the slowest requests

```bash
harvey entries capture.har --sort-by time --limit 10 --json --quiet | head
```

### Audit third-party domains

```bash
harvey domains capture.har --sort-by bytes --json --quiet | jq '.domains[] | {domain, request_count, total_bytes, avg_time_ms}'
```

### Extract all API calls

```bash
harvey entries capture.har --filter-url "/api/" --json --quiet
```

### Check for missing resources (404s)

```bash
harvey entries capture.har --filter-status 404 --json --quiet
```

### Get a quick overview before drilling in

```bash
# Step 1: big picture
harvey analyze capture.har --json --quiet

# Step 2: check for errors
harvey entries capture.har --filter-status 500 --json --quiet

# Step 3: drill into the slowest entry in detail
harvey inspect capture.har --entry $(harvey entries capture.har --sort-by time --limit 1 --json --quiet | jq -r '.entry_index')

# Step 4: audit third parties
harvey domains capture.har --sort-by requests --json --quiet
```

## JSON Output Conventions

1. **All JSON output includes `"format_version": "1.0"`** for forward
   compatibility.
2. **Entries output is JSONL (NDJSON)** — one complete JSON object per line.
   Parse line-by-line, do not treat as a JSON array.
3. **`_computed` namespace** — additive fields not in the HAR spec. Always
   under `_computed` to avoid collisions with real HAR fields.
4. **Exit code 4 for empty filters** — distinct from errors. An agent should
   handle this gracefully ("nothing matched" vs "something failed").
5. **Stdout = data, stderr = diagnostics** — with `--quiet`, only data goes
   to stdout. Safe to pipe.

## Project Structure

```
~/.projects/personal/harvey/
├── Cargo.toml
├── clippy.toml
├── CONSTITUTION.md
├── schemas/                    # JSON Schema files for each command
├── src/
│   ├── lib.rs                  # Library — re-exports `har` module
│   ├── main.rs                 # Binary — CLI dispatch + tracing + exit codes
│   ├── cli.rs                  # GlobalArgs (--verbose, --quiet, --json)
│   ├── output.rs               # OutputMode enum (Human / Json)
│   ├── commands/               # Subcommand implementations
│   │   ├── analyze.rs
│   │   ├── entries.rs
│   │   ├── domains.rs
│   │   └── schema.rs
│   └── har/                    # Core library
│       ├── types.rs            # HAR 1.2 data structures
│       ├── parser.rs           # Load + validate .har files
│       ├── stats.rs            # Aggregate statistics
│       └── filter.rs           # EntryPredicate + filter logic
└── tests/
    ├── integration.rs
    └── fixtures/example.har
```
