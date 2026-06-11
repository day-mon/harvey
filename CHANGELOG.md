# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial release of `harvey` — AI-native HAR file analyzer
- `analyze` command for aggregate statistics and distributions
- `entries` command for listing and filtering HTTP entries
- `domains` command for per-domain breakdown
- `endpoints` command for deduplicated API endpoint summary
- `capture` command for live Chrome DevTools Protocol capture
- `inspect` command for detailed single-entry view
- `schema` command for JSON output schema introspection
- `--json` flag on every command for machine-readable output
- CI/CD pipelines with format, clippy, and test checks
- Pre-commit hooks via rusty-hook
- Git-derived versioning via vergen-git2
