# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Fixed

- Saturate GNU sleep-compatible oversized finite durations instead of rejecting them.

- Match GNU `sleep` by requiring lowercase unit suffixes on infinite durations.

- Accept duration decimals in either the current numeric locale or the C locale, matching GNU sleep.
- Accept GNU sleep-compatible hexadecimal floating-point durations.
- Reject uppercase duration suffixes to match GNU sleep.
- Accept GNU sleep-compatible leading `+` and `infinity` duration forms.
- Accept GNU sleep-compatible fractional and scientific-notation durations, plus `inf` for indefinite sleeps.
- Preserve the absolute `--until` deadline so argument parsing and setup time cannot extend the sleep.
- Accept negative UTC offsets passed as a separate token to `--until`.
- Round the progress countdown up so it does not display one second less than the remaining sleep time.
- Return overflow errors instead of panicking for out-of-range durations, timestamps, and sleep deadlines.
- Reject out-of-range datetime numeric fields before narrowing them to their validated field types.
- Prevent `--until` from panicking on invalid datetime input containing multibyte UTF-8 characters.
- Resolve local `--until` deadlines using the UTC offset at the target local datetime, including time-only and `tomorrow` deadlines across daylight-saving transitions.
