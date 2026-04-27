<h1 align=center>Advanced Sleep

<code>asleep</code>

</h1>

<div align="center">
</div>
<div align="center">
    <img src="https://img.shields.io/badge/-Linux-9C2A91.svg?style=flat&logo=linux&logoColor=white" alt="Linux" title="Supported Platform: Linux">&thinsp;
    <img src="https://img.shields.io/badge/-macOS-red.svg?style=flat&logo=apple&logoColor=white" alt="macOS" title="Supported Platform: macOS">
    <img src="https://img.shields.io/badge/-Windows-6E46A2.svg?style=flat&logo=windows-11&logoColor=white" alt="Windows" title="Supported Platform: Windows">&thinsp;
</div>

# asleep

An advanced, suspend-aware, and GNU sleep-compatible sleep utility with live countdown and flexible datetime parsing.

## Features

- **Live Countdown**: Shows remaining time in the terminal (auto-disabled if not a TTY).
- **Suspend-aware**: Compensates for system suspend/resume by polling `SystemTime` (opt out with `-m, --monotonic`).
- **Sleep Until**: Support for sleeping until a specific datetime.
- **Zero dependencies**: ... Except for the platforms C runtime for signal handling.

## Demo

![asleep demo](docs/demo.gif)

## Usage
```bash
asleep 1h30m
asleep 5m
asleep 90
asleep --until tomorrow 8am
```

### Flags

- `-h`, `--help`: Show help message.
- `-n`, `--no-progress`: Disable live countdown display.
- `-m`, `--monotonic`: Use a monotonic clock (does not count time spent in suspend).
- `-u`, `--until DATETIME`: Sleep until the specified datetime.

### Datetime Formats

`asleep` supports several formats for the `--until` flag:

- **Unix Timestamp**: `@1234567890` (seconds since epoch).
- **Time Only**: `HH:MM`, `HH:MM:SS`, or `HHam`/`HHpm`. Supports both 24-hour and 12-hour formats (with `am`/`pm` suffix). Interpreted as local time. If the time has already passed today, tomorrow is assumed.
- **UTC Time**: `HH:MMZ` or `HH:MM:SS UTC`.
- **Offset Time**: `HH:MM+02:00`, `HH:MM-05:00`, or `HH:MM+0530`.
- **Tomorrow**: `tomorrow` (midnight) or `tomorrow 10:00`.
- **Full Datetime**: `YYYY-MM-DD HH:MM:SS`. Date part can be combined with any of the time formats above.

### Examples

```bash
asleep --until 23:59
asleep --until 11:59pm
asleep --until 6pm
asleep --until 06:00Z
asleep --until tomorrow 8am
asleep --until "2026-12-25 10:00:00+02:00"
```

## Install

### 📥 Prebuilt binaries

Prebuilt binaries for Linux, macOS, and Windows can be found on [the releases page](https://github.com/CramBL/asleep/releases).

Install the latest version with the standalone installer:

```bash
# macOS and Linux.
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/CramBL/asleep/releases/latest/download/asleep-installer.sh | sh
```

```powershell
# Windows.
powershell -ExecutionPolicy Bypass -c "irm https://github.com/CramBL/asleep/releases/latest/download/asleep-installer.ps1 | iex"
```

### With `homebrew`

```console
brew install CramBL/tap/asleep
```

### With `cargo`

`asleep` is available on [crates.io](https://crates.io/crates/asleep).

```console
cargo install asleep
```

### From Source

Ensure you have [Rust](https://rustup.rs/) installed, then run:

```bash
git clone https://github.com/CramBL/asleep.git
cd asleep
cargo build --release
```
