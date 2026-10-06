use crate::args::{Config, SleepTarget};
use crate::display::{TerminalGuard, update_progress_seconds};
use crate::duration::Seconds;
use crate::signal;
use std::io::{self, IsTerminal, Write};
use std::thread;
#[cfg(not(windows))]
use std::time::Instant;
use std::time::{Duration, SystemTime};

pub fn run_sleep(config: Config) -> i32 {
    let Config {
        target,
        poll_interval,
        show_progress,
        suspend_aware,
    } = config;
    let deadline = match build_deadline(target, suspend_aware) {
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
            Deadline::SuspendAware(deadline) => deadline
                .checked_duration_since(SuspendAwareInstant::now())
                .unwrap_or(Duration::ZERO),
            Deadline::Monotonic(deadline) => deadline
                .checked_duration_since(MonotonicInstant::now())
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

fn build_deadline(target: SleepTarget, suspend_aware: bool) -> Option<Deadline> {
    match target {
        SleepTarget::Deadline(deadline) => Some(Deadline::Wall(deadline)),
        SleepTarget::Duration(duration) if suspend_aware => SuspendAwareInstant::now()
            .checked_add(duration)
            .map(Deadline::SuspendAware),
        SleepTarget::Duration(duration) => MonotonicInstant::now()
            .checked_add(duration)
            .map(Deadline::Monotonic),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SuspendAwareInstant(Duration);

impl SuspendAwareInstant {
    fn now() -> Self {
        Self(suspend_aware_now())
    }

    fn checked_add(self, duration: Duration) -> Option<Self> {
        self.0.checked_add(duration).map(Self)
    }

    fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
        self.0.checked_sub(earlier.0)
    }
}

#[cfg(target_os = "linux")]
fn suspend_aware_now() -> Duration {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: `ts` is a valid writable timespec and CLOCK_BOOTTIME is provided
    // by Linux specifically for monotonic elapsed time that includes suspend.
    let result = unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut ts) };
    assert_eq!(result, 0, "CLOCK_BOOTTIME is unavailable");
    Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32)
}

#[cfg(target_os = "macos")]
fn suspend_aware_now() -> Duration {
    #[repr(C)]
    struct MachTimebaseInfo {
        numer: u32,
        denom: u32,
    }

    unsafe extern "C" {
        fn mach_continuous_time() -> u64;
        fn mach_timebase_info(info: *mut MachTimebaseInfo) -> i32;
    }

    static TIMEBASE: std::sync::OnceLock<(u64, u64)> = std::sync::OnceLock::new();
    let (numer, denom) = *TIMEBASE.get_or_init(|| {
        let mut info = MachTimebaseInfo { numer: 0, denom: 0 };
        // SAFETY: `info` has the C layout expected by mach_timebase_info and is
        // valid for the duration of the call.
        let result = unsafe { mach_timebase_info(&mut info) };
        assert_eq!(result, 0, "mach_timebase_info failed");
        assert_ne!(
            info.denom, 0,
            "mach_timebase_info returned a zero denominator"
        );
        (u64::from(info.numer), u64::from(info.denom))
    });

    // SAFETY: mach_continuous_time takes no arguments and returns a monotonic
    // tick counter that continues advancing while the machine is suspended.
    let ticks = unsafe { mach_continuous_time() };
    let nanos = u128::from(ticks) * u128::from(numer) / u128::from(denom);
    Duration::new(
        (nanos / 1_000_000_000) as u64,
        (nanos % 1_000_000_000) as u32,
    )
}

#[cfg(windows)]
fn suspend_aware_now() -> Duration {
    use windows_sys::Win32::System::WindowsProgramming::QueryInterruptTimePrecise;

    let mut interrupt_time = 0;
    // SAFETY: `interrupt_time` is a valid writable u64. QueryInterruptTimePrecise
    // cannot fail and reports suspend-aware interrupt time in 100 ns units.
    unsafe { QueryInterruptTimePrecise(&mut interrupt_time) };

    let seconds = interrupt_time / 10_000_000;
    let nanos = ((interrupt_time % 10_000_000) * 100) as u32;
    Duration::new(seconds, nanos)
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
fn suspend_aware_now() -> Duration {
    static ORIGIN: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    ORIGIN.get_or_init(Instant::now).elapsed()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct MonotonicInstant(MonotonicInstantInner);

#[cfg(windows)]
type MonotonicInstantInner = Duration;

#[cfg(not(windows))]
type MonotonicInstantInner = Instant;

impl MonotonicInstant {
    fn now() -> Self {
        Self(monotonic_now())
    }

    fn checked_add(self, duration: Duration) -> Option<Self> {
        self.0.checked_add(duration).map(Self)
    }

    fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
        #[cfg(windows)]
        {
            self.0.checked_sub(earlier.0)
        }
        #[cfg(not(windows))]
        {
            self.0.checked_duration_since(earlier.0)
        }
    }
}

#[cfg(windows)]
fn monotonic_now() -> Duration {
    use windows_sys::Win32::System::WindowsProgramming::QueryUnbiasedInterruptTimePrecise;

    let mut interrupt_time = 0;
    // SAFETY: `interrupt_time` is a valid writable u64.
    // QueryUnbiasedInterruptTimePrecise cannot fail and reports active-system
    // interrupt time in 100 ns units, excluding sleep and hibernation.
    unsafe { QueryUnbiasedInterruptTimePrecise(&mut interrupt_time) };

    let seconds = interrupt_time / 10_000_000;
    let nanos = ((interrupt_time % 10_000_000) * 100) as u32;
    Duration::new(seconds, nanos)
}

#[cfg(not(windows))]
fn monotonic_now() -> Instant {
    Instant::now()
}

#[derive(Debug, PartialEq)]
enum Deadline {
    Wall(SystemTime),
    SuspendAware(SuspendAwareInstant),
    Monotonic(MonotonicInstant),
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
        assert_eq!(
            build_deadline(SleepTarget::Deadline(target), true),
            Some(Deadline::Wall(target)),
        );
    }

    #[test]
    fn relative_suspend_aware_duration_uses_monotonic_boot_clock() {
        let deadline = build_deadline(SleepTarget::Duration(Duration::from_secs(10)), true);

        assert!(matches!(deadline, Some(Deadline::SuspendAware(_))));
    }

    #[test]
    fn relative_monotonic_duration_uses_suspend_excluding_clock() {
        let deadline = build_deadline(SleepTarget::Duration(Duration::from_secs(10)), false);

        assert!(matches!(deadline, Some(Deadline::Monotonic(_))));
    }
}
