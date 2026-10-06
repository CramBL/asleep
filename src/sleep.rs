use crate::args::{Config, SleepTarget};
use crate::display::{TerminalGuard, update_progress_seconds};
use crate::duration::Seconds;
use crate::signal;
use std::io::{self, IsTerminal, Write};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

pub fn run_sleep(config: Config) -> i32 {
    let Config {
        target,
        poll_interval,
        show_progress,
        suspend_aware,
    } = config;
    let deadline = match build_deadline(target, suspend_aware, SystemTime::now(), Instant::now()) {
        Some(deadline) => deadline,
        None => return deadline_overflow(),
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

        if deadline == Deadline::Infinite {
            thread::sleep(poll_duration);
            continue;
        }

        let remaining = match deadline {
            Deadline::Wall(deadline) => deadline
                .duration_since(SystemTime::now())
                .unwrap_or(Duration::ZERO),
            Deadline::Monotonic(deadline) => deadline
                .checked_duration_since(Instant::now())
                .unwrap_or(Duration::ZERO),
            Deadline::Infinite => unreachable!(),
        };

        if remaining.is_zero() {
            break;
        }

        if show_progress && is_tty {
            update_progress_seconds(display_seconds(remaining));
        }

        let sleep_time = remaining.min(poll_duration);
        if sleep_time.is_zero() {
            break;
        }

        thread::sleep(sleep_time);
    }
    exit_code
}

fn build_deadline(
    target: SleepTarget,
    suspend_aware: bool,
    wall_now: SystemTime,
    monotonic_now: Instant,
) -> Option<Deadline> {
    match target {
        SleepTarget::Deadline(deadline) => Some(Deadline::Wall(deadline)),
        SleepTarget::Duration(duration) if suspend_aware => {
            wall_now.checked_add(duration).map(Deadline::Wall)
        }
        SleepTarget::Duration(duration) => {
            monotonic_now.checked_add(duration).map(Deadline::Monotonic)
        }
        SleepTarget::Infinite => Some(Deadline::Infinite),
    }
}

fn display_seconds(remaining: Duration) -> Seconds {
    Seconds(
        remaining
            .as_secs()
            .saturating_add(u64::from(remaining.subsec_nanos() != 0)),
    )
}

#[derive(Debug, PartialEq)]
enum Deadline {
    Wall(SystemTime),
    Monotonic(Instant),
    Infinite,
}

fn deadline_overflow() -> i32 {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(b"Error: Sleep duration is out of range\n");
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_seconds_rounds_up_fractional_seconds() {
        assert_eq!(display_seconds(Duration::ZERO).0, 0);
        assert_eq!(display_seconds(Duration::from_secs(5)).0, 5);
        assert_eq!(display_seconds(Duration::from_millis(4999)).0, 5);
        assert_eq!(display_seconds(Duration::from_millis(1)).0, 1);
    }

    #[test]
    fn absolute_deadline_is_not_rebased_when_sleep_starts() {
        let parsed_at = std::time::UNIX_EPOCH + Duration::from_secs(100);
        let target = parsed_at + Duration::from_secs(10);
        let sleep_started_at = parsed_at + Duration::from_secs(1);

        assert_eq!(
            build_deadline(
                SleepTarget::Deadline(target),
                true,
                sleep_started_at,
                Instant::now(),
            ),
            Some(Deadline::Wall(target)),
        );
    }
}
