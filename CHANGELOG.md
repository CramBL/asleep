# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Fixed

- Accept negative UTC offsets passed as a separate token to `--until`.
- Round the progress countdown up so it does not display one second less than the remaining sleep time.
- Return overflow errors instead of panicking for out-of-range durations, timestamps, and sleep deadlines.
- Reject out-of-range datetime numeric fields before narrowing them to their validated field types.
- Prevent `--until` from panicking on invalid datetime input containing multibyte UTF-8 characters.
- Resolve local `--until` deadlines using the UTC offset at the target local datetime, including time-only and `tomorrow` deadlines across daylight-saving transitions.
