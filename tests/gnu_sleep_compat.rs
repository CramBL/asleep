#![cfg(unix)]

use rstest::rstest;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

// Reference behavior is derived from GNU coreutils' tests/misc/sleep.sh and
// checked differentially against an installed GNU sleep. The upstream snapshot
// used when this suite was introduced was coreutils master at
// e6c09ec0b1b590a77a3736c23bf888c4cb13f438.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Success,
    Failure,
    TimedOut,
}

fn gnu_sleep() -> Option<PathBuf> {
    if let Some(path) = env::var_os("GNU_SLEEP").map(PathBuf::from)
        && is_gnu_sleep(&path)
    {
        return Some(path);
    }

    ["sleep", "gsleep"]
        .into_iter()
        .map(PathBuf::from)
        .find(|path| is_gnu_sleep(path))
}

fn is_gnu_sleep(path: &Path) -> bool {
    let Ok(output) = Command::new(path).arg("--version").output() else {
        return false;
    };

    output.status.success() && String::from_utf8_lossy(&output.stdout).contains("GNU coreutils")
}

fn run_with_timeout(program: &Path, args: &[&str], timeout: Duration) -> Outcome {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("failed to run {}: {error}", program.display()));
    let deadline = Instant::now() + timeout;

    loop {
        if let Some(status) = child.try_wait().expect("failed to poll child process") {
            return if status.success() {
                Outcome::Success
            } else {
                Outcome::Failure
            };
        }

        if Instant::now() >= deadline {
            let _ = child.kill();
            child
                .wait()
                .expect("failed to reap timed-out child process");
            return Outcome::TimedOut;
        }

        thread::sleep(Duration::from_millis(5));
    }
}

fn compare_with_gnu(args: &[&str], timeout: Duration) -> Option<(Outcome, Outcome)> {
    let gnu = gnu_sleep()?;
    let asleep = PathBuf::from(assert_cmd::cargo::cargo_bin!("asleep"));

    let reference = run_with_timeout(&gnu, args, timeout);
    let mut asleep_args = args.to_vec();
    asleep_args.push("--no-progress");
    let actual = run_with_timeout(&asleep, &asleep_args, timeout);

    Some((reference, actual))
}

fn assert_compatible(args: &[&str], expected: Outcome, timeout: Duration) {
    let Some((reference, actual)) = compare_with_gnu(args, timeout) else {
        return;
    };

    assert_eq!(
        reference, expected,
        "unexpected GNU sleep behavior for {args:?}"
    );
    assert_eq!(
        actual, reference,
        "asleep differs from GNU sleep for {args:?}"
    );
}

#[rstest]
#[case::invalid(&["invalid"])]
#[case::negative(&["-1"])]
#[case::uppercase_suffix(&["42D"])]
#[case::extra_suffix_text(&["42d", "42day"])]
#[case::nan(&["nan"])]
#[case::empty(&[""])]
fn rejects_the_same_invalid_operands_as_gnu_sleep(#[case] args: &[&str]) {
    assert_compatible(args, Outcome::Failure, Duration::from_millis(250));
}

#[rstest]
#[case::zero(&["0"])]
#[case::fractional(&["0.001"])]
#[case::scientific(&["1e-3"])]
#[case::leading_plus(&["+0.001"])]
#[case::multiple_zero_units(&["0d", "0h", "0m", "0s"])]
fn accepts_the_same_short_numeric_operands_as_gnu_sleep(#[case] args: &[&str]) {
    assert_compatible(args, Outcome::Success, Duration::from_millis(500));
}

#[rstest]
#[case::multiple_units(&["1d", "2h", "3m", "4s"])]
#[case::inf(&["inf"])]
#[case::infinity(&["infinity"])]
fn accepts_the_same_long_running_operands_as_gnu_sleep(#[case] args: &[&str]) {
    assert_compatible(args, Outcome::TimedOut, Duration::from_millis(100));
}

#[rstest]
#[case::hex_fraction(&["0x.002p1"])]
#[case::hex_digit_not_day_suffix(&["0x0.01d"])]
fn accepts_gnu_hexadecimal_floating_point_operands(#[case] args: &[&str]) {
    // These are taken directly from GNU coreutils' sleep parameter tests.
    assert_compatible(args, Outcome::Success, Duration::from_millis(500));
}
