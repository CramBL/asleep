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

fn run_with_timeout(
    program: &Path,
    args: &[&str],
    timeout: Duration,
    locale: Option<&str>,
) -> Outcome {
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(locale) = locale {
        command.env("LC_ALL", locale);
    }

    let mut child = command
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

fn compare_with_gnu(
    args: &[&str],
    timeout: Duration,
    locale: Option<&str>,
) -> Option<(Outcome, Outcome)> {
    let gnu = gnu_sleep()?;
    let asleep = PathBuf::from(assert_cmd::cargo::cargo_bin!("asleep"));

    let reference = run_with_timeout(&gnu, args, timeout, locale);
    let mut asleep_args = Vec::with_capacity(args.len() + 1);
    asleep_args.push("--no-progress");
    asleep_args.extend_from_slice(args);
    let actual = run_with_timeout(&asleep, &asleep_args, timeout, locale);

    Some((reference, actual))
}

fn assert_compatible(args: &[&str], expected: Outcome, timeout: Duration) {
    let Some((reference, actual)) = compare_with_gnu(args, timeout, None) else {
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
#[case::uppercase_infinite_suffix(&["INFD"])]
#[case::extra_suffix_text(&["42d", "42day"])]
#[case::nan(&["nan"])]
#[case::empty(&[""])]
#[case::missing_operand(&[])]
#[case::trailing_whitespace(&["0.001 "])]
#[case::negative_decimal_underflow(&["--", "-1e-9999"])]
#[case::negative_hex_underflow(&["--", "-0x1p-99999"])]
#[case::separated_positive_sign(&["+ 0"])]
#[case::separated_positive_infinity_sign(&["+ inf"])]
fn rejects_the_same_invalid_operands_as_gnu_sleep(#[case] args: &[&str]) {
    assert_compatible(args, Outcome::Failure, Duration::from_millis(250));
}

#[rstest]
#[case::zero(&["0"])]
#[case::fractional(&["0.001"])]
#[case::scientific(&["1e-3"])]
#[case::leading_plus(&["+0.001"])]
#[case::leading_whitespace(&[" 0.001"])]
#[case::multiple_zero_units(&["0d", "0h", "0m", "0s"])]
#[case::end_of_options(&["--", "0"])]
#[case::bare_end_of_options(&["--"])]
#[case::negative_zero(&["--", "-0"])]
#[case::negative_zero_with_suffix(&["--", "-0s"])]
#[case::negative_zero_fraction(&["--", "-0.0"])]
#[case::negative_zero_hex(&["--", "-0x0p0"])]
#[case::positive_decimal_underflow(&["1e-9999"])]
#[case::positive_hex_underflow(&["0x1p-1075"])]
#[case::extreme_hex_underflow(&["0x1p-999999999999999999999999"])]
#[case::extreme_zero_hex_exponent(&["0x0p999999999999999999999999"])]
fn accepts_the_same_short_numeric_operands_as_gnu_sleep(#[case] args: &[&str]) {
    assert_compatible(args, Outcome::Success, Duration::from_millis(500));
}

#[rstest]
#[case::multiple_units(&["1d", "2h", "3m", "4s"])]
#[case::inf(&["inf"])]
#[case::infinity(&["infinity"])]
#[case::uppercase_inf(&["INF"])]
#[case::uppercase_inf_with_lowercase_suffix(&["INFd"])]
#[case::positive_inf(&["+inf"])]
#[case::positive_infinity(&["+infinity"])]
#[case::positive_inf_with_suffix(&["+INFd"])]
#[case::huge_scientific(&["1e400"])]
#[case::beyond_u64_seconds(&["18446744073709551616"])]
#[case::huge_unit_scaled_integer(&["9999999999999999999d"])]
#[case::hex_overflow(&["0x1p1024"])]
#[case::extreme_hex_overflow(&["0x1p999999999999999999999999"])]
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

#[rstest]
#[case::help_shortest("--h")]
#[case::help_prefix("--he\u{6c}")]
#[case::help_full("--help")]
#[case::version_shortest("--v")]
#[case::version_prefix("--vers")]
#[case::version_full("--version")]
fn accepts_same_standard_option_abbreviations_as_gnu_sleep(#[case] option: &str) {
    let Some(gnu) = gnu_sleep() else {
        return;
    };
    let asleep = PathBuf::from(assert_cmd::cargo::cargo_bin!("asleep"));
    let timeout = Duration::from_millis(250);

    let reference = run_with_timeout(&gnu, &[option], timeout, None);
    let actual = run_with_timeout(&asleep, &[option], timeout, None);

    assert_eq!(
        reference,
        Outcome::Success,
        "unexpected GNU sleep behavior for {option}"
    );
    assert_eq!(
        actual, reference,
        "asleep differs from GNU sleep for {option}"
    );
}

fn comma_decimal_locale() -> Option<String> {
    let output = Command::new("locale").arg("-a").output().ok()?;
    if !output.status.success() {
        return None;
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .find(|locale| {
            Command::new("locale")
                .arg("decimal_point")
                .env("LC_ALL", locale)
                .output()
                .ok()
                .filter(|output| output.status.success())
                .is_some_and(|output| {
                    String::from_utf8_lossy(&output.stdout)
                        .trim()
                        .trim_matches('"')
                        == ","
                })
        })
        .map(str::to_owned)
}

#[rstest]
#[case::current_locale_decimal("0,001")]
#[case::c_locale_decimal("0.001")]
fn accepts_current_and_c_locale_decimal_operands(#[case] operand: &str) {
    let Some(locale) = comma_decimal_locale() else {
        return;
    };
    let args = &[operand];
    let Some((reference, actual)) =
        compare_with_gnu(args, Duration::from_millis(500), Some(&locale))
    else {
        return;
    };

    assert_eq!(
        reference,
        Outcome::Success,
        "unexpected GNU sleep behavior for {args:?} under {locale}"
    );
    assert_eq!(
        actual, reference,
        "asleep differs from GNU sleep for {args:?} under {locale}"
    );
}
