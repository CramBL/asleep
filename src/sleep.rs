use crate::args::Config;
use crate::display::{TerminalGuard, update_progress_seconds};
use crate::duration::Seconds;
use crate::signal;
use std::io::{self, IsTerminal, Write};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

pub fn run_sleep(config: Config) -> i32 {
    let Config {
        duration,
        poll_interval,
        show_progress,
        suspend_aware,
    } = config;
    let deadline = if suspend_aware {
        match SystemTime::now().checked_add(duration) {
            Some(deadline) => Deadline::Wall(deadline),
            None => return deadline_overflow(),
        }
    } else {
        match Instant::now().checked_add(duration) {
            Some(deadline) => Deadline::Monotonic(deadline),
            None => return deadline_overflow(),
        }
    };

    let is_tty = std::io::stdout().is_terminal();
    let _guard = if show_progress && is_tty {
        Some(TerminalGuard::new())
    } else {
        None
    };

    let poll_duration = if show_progress && is_tty {
        Duration::from_millis(200)
    } else {
        Duration::from_secs(poll_interval.0.0)
    };

    let mut exit_code = 0;
    loop {
        if signal::stop_signal_received() {
            exit_code = 130;
            break;
        }

        let remaining = match deadline {
            Deadline::Wall(deadline) => deadline
                .duration_since(SystemTime::now())
                .unwrap_or(Duration::ZERO),
            Deadline::Monotonic(deadline) => deadline
                .checked_duration_since(Instant::now())
                .unwrap_or(Duration::ZERO),
        };

        if remaining.is_zero() {
            break;
        }

        if show_progress && is_tty {
            update_progress_seconds(Seconds(remaining.as_secs()));
        }

        let sleep_time = remaining.min(poll_duration);
        if sleep_time.is_zero() {
            break;
        }

        thread::sleep(sleep_time);
    }
    exit_code
}

enum Deadline {
    Wall(SystemTime),
    Monotonic(Instant),
}

fn deadline_overflow() -> i32 {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(b"Error: Sleep duration is out of range\n");
    1
}
