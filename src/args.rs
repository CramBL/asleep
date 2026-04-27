use crate::duration::PollInterval;
use crate::parse::{parse_datetime, parse_duration};
use std::env;
use std::io::{self, Write};
use std::time::{Duration, SystemTime};

pub struct Config {
    pub duration: Duration,
    pub poll_interval: PollInterval,
    pub show_progress: bool,
    pub suspend_aware: bool,
}

pub fn parse() -> Config {
    let mut args = env::args().skip(1).peekable();
    if args.peek().is_none() {
        die_with_usage(b"No arguments provided");
    }

    let mut show_progress = true;
    let mut suspend_aware = true;
    let mut duration_strs: Vec<String> = Vec::new();
    let mut until_parts: Vec<String> = Vec::new();
    let mut until_flag_seen = false;
    let poll_interval = PollInterval(crate::duration::Seconds(1));

    while let Some(s) = args.next() {
        match s.as_str() {
            "-h" | "--help" => {
                print_usage(&mut io::stdout().lock());
                std::process::exit(0);
            }
            "-V" | "--version" => {
                let mut stdout = io::stdout().lock();
                let _ = writeln!(stdout, "asleep {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-n" | "--no-progress" => show_progress = false,
            "-m" | "--monotonic" => suspend_aware = false,
            "-u" | "--until" => {
                until_flag_seen = true;
                while let Some(next) = args.peek() {
                    if next.starts_with('-') {
                        break;
                    }
                    until_parts.push(args.next().unwrap());
                }
            }
            _ if s.starts_with('-') => {
                let mut stderr = io::stderr().lock();
                let _ = stderr.write_all(b"Error: Unknown flag ");
                let _ = stderr.write_all(s.as_bytes());
                let _ = stderr.write_all(b"\n");
                print_usage(&mut stderr);
                std::process::exit(1);
            }
            _ => {
                duration_strs.push(s);
            }
        }
    }

    let until_val = if until_flag_seen {
        if until_parts.is_empty() {
            die(b"--until requires a value");
        }
        Some(until_parts.join(" "))
    } else {
        None
    };

    if !duration_strs.is_empty() && until_val.is_some() {
        die(b"Cannot provide both duration and --until");
    }

    let duration = if let Some(s) = until_val {
        let now = SystemTime::now();
        match parse_datetime(&s, now) {
            Ok(target) => target.duration_since(now).unwrap_or(Duration::ZERO),
            Err(e) => die_parse(b"datetime", &s, e.as_str()),
        }
    } else if !duration_strs.is_empty() {
        let mut total = Duration::ZERO;
        for s in duration_strs {
            match parse_duration(&s) {
                Ok(d) => total += d,
                Err(e) => die_parse(b"duration", &s, e.as_str()),
            }
        }
        total
    } else {
        die_with_usage(b"No duration or --until provided");
    };

    Config {
        duration,
        poll_interval,
        show_progress,
        suspend_aware,
    }
}

fn die(msg: &[u8]) -> ! {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(b"Error: ");
    let _ = stderr.write_all(msg);
    let _ = stderr.write_all(b"\n");
    std::process::exit(1);
}

fn die_with_usage(msg: &[u8]) -> ! {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(b"Error: ");
    let _ = stderr.write_all(msg);
    let _ = stderr.write_all(b"\n");
    print_usage(&mut stderr);
    std::process::exit(1);
}

fn die_parse(label: &[u8], input: &str, err_msg: &str) -> ! {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(b"Error parsing ");
    let _ = stderr.write_all(label);
    let _ = stderr.write_all(b" '");
    let _ = stderr.write_all(input.as_bytes());
    let _ = stderr.write_all(b"': ");
    let _ = stderr.write_all(err_msg.as_bytes());
    let _ = stderr.write_all(b"\n");
    std::process::exit(1);
}

fn print_usage(writer: &mut impl Write) {
    const HEADER: &str = concat!(
        "asleep ",
        env!("CARGO_PKG_VERSION"),
        "\n",
        env!("CARGO_PKG_AUTHORS"),
        "\n\n",
        "An advanced, GNU sleep-compatible sleep utility.\n\n",
        "Usage: asleep [FLAGS] <duration ... | --until DATETIME>\n\n",
        "Flags:\n",
        "  -h, --help           Show this help message\n",
        "  -V, --version        Show version information\n",
        "  -n, --no-progress    Disable the live countdown display\n",
        "  -m, --monotonic      Use a monotonic clock (does not count time spent in suspend)\n",
        "  -u, --until VALUE    Sleep until specified datetime\n\n",
        "Datetime formats:\n",
        "  @1234567890          Unix timestamp\n",
        "  HH:MM[:SS]           Time-only (interpreted as local time)\n",
        "  HH[[:MM]:SS]am/pm    12-hour time format\n",
        "  HH:MM[:SS]Z          Time-only in UTC\n",
        "  HH:MM[:SS]\u{00B1}HH[:MM]   Time-only with explicit offset\n",
        "  tomorrow [TIME]      Tomorrow at specified time (local or with suffix)\n",
        "  YYYY-MM-DD [TIME]    Specific date and time\n\n",
        "Notes on Timezones:\n",
        "  - By default, times are interpreted in your system's local timezone.\n",
        "  - Append 'Z' or 'UTC' for Coordinated Universal Time.\n",
        "  - Append an offset like '+02:00' or '-05:00' for a specific timezone.\n",
        "  - If a time-only deadline has already passed today, tomorrow is assumed.\n\n",
        "Example:\n",
        "  asleep 1h30m\n",
        "  asleep --until 23:59\n",
        "  asleep --until 6pm\n",
        "  asleep --until 11:30am\n",
        "  asleep --until 06:00Z\n",
        "  asleep --until tomorrow 08:00\n",
        "  asleep --until tomorrow 8am\n",
        "  asleep --until 2026-04-23 10:00+02:00\n"
    );
    let _ = writer.write_all(HEADER.as_bytes());
}
