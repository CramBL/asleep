# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Fixed

- Reject out-of-range datetime numeric fields before narrowing them to their validated field types.
- Prevent `--until` from panicking on invalid datetime input containing multibyte UTF-8 characters.
- Resolve local `--until` deadlines using the UTC offset at the target local datetime, including time-only and `tomorrow` deadlines across daylight-saving transitions.
