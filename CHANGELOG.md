# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Fixed

- Resolve local `--until` deadlines using the UTC offset at the target local datetime, including time-only and `tomorrow` deadlines across daylight-saving transitions.
- Resolve the Cargo target directory in Unix shell tests with `cargo metadata`, so custom target directory locations are supported.
