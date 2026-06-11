# harvey

AI-native HAR (HTTP Archive) file analyzer. Analyze browser DevTools captures
with machine-readable output designed for both humans and autonomous AI agents.

## Install

```bash
cargo install harvey
```

## Usage

```bash
harvey analyze capture.har        # summary stats
harvey entries capture.har        # list and filter entries
harvey domains capture.har        # per-domain breakdown
harvey endpoints capture.har      # deduplicated API endpoints
harvey inspect capture.har        # full detail on a single entry
harvey capture --url https://...  # live capture via Chrome CDP
harvey schema entries             # inspect JSON output format
```

Every command supports `--json` for machine-readable output.

## Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | File not found |
| 3 | Invalid HAR structure |
| 4 | No results matched |
