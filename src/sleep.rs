use crate::args::Config;
use crate::display::{TerminalGuard, update_progress_seconds};
use crate::duration::Seconds;
use crate::signal;
use std::io::IsTerminal;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

pub fn run_sleep(config: Config) -> i32 {
    let Config {
        duration,
        poll_interval,
        show_progress,
        suspend_aware,
    } = config;
    let deadline_wall = SystemTime::now() + duration;
    let deadline_mono = Instant::now() + duration;

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

        let remaining = if suspend_aware {
            deadline_wall
                .duration_since(SystemTime::now())
                .unwrap_or(Duration::ZERO)
        } else {
            deadline_mono
                .checked_duration_since(Instant::now())
                .unwrap_or(Duration::ZERO)
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
