use assert_cmd::Command;
use predicates::prelude::*;
use std::time::Duration;

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("asleep").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: asleep"));
}

#[test]
fn test_version() {
    let version = env!("CARGO_PKG_VERSION");
    let expected = format!("asleep {}", version);

    let mut cmd = Command::cargo_bin("asleep").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with(expected.clone()));

    let mut cmd = Command::cargo_bin("asleep").unwrap();
    cmd.arg("-V")
        .assert()
        .success()
        .stdout(predicate::str::starts_with(expected));
}

#[test]
fn test_invalid_duration() {
    let mut cmd = Command::cargo_bin("asleep").unwrap();
    cmd.arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Error parsing duration"));
}

#[test]
fn test_duration_sleep() {
    let mut cmd = Command::cargo_bin("asleep").unwrap();
    let now = std::time::Instant::now();
    cmd.arg("1s").arg("--no-progress").assert().success();
    let elapsed = now.elapsed();
    assert!(elapsed >= Duration::from_millis(1000));
    assert!(elapsed < Duration::from_millis(1500));
}

#[test]
fn test_multiple_durations() {
    let mut cmd = Command::cargo_bin("asleep").unwrap();
    let now = std::time::Instant::now();
    cmd.args(["1s", "2s", "1s"])
        .arg("--no-progress")
        .assert()
        .success();
    let elapsed = now.elapsed();
    assert!(elapsed >= Duration::from_millis(4000));
    assert!(elapsed < Duration::from_millis(4500));
}

#[test]
fn test_until_past_fails() {
    let mut cmd = Command::cargo_bin("asleep").unwrap();
    cmd.args(["--until", "@0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("in the past"));
}

#[test]
fn test_until_timestamp() {
    let mut cmd = Command::cargo_bin("asleep").unwrap();
    let future = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 1;

    cmd.args(["--until", &format!("@{}", future)])
        .arg("--no-progress")
        .assert()
        .success();
}

fn assert_interrupted(mut cmd: Command, timeout: Duration) {
    let assert = cmd.timeout(timeout).assert();
    if cfg!(windows) {
        // On Windows, timeout might result in 130 (our signal handler) or a default kill code
        assert.code(predicate::in_iter([130, 0xC000013Au32 as i32, 1]));
    } else {
        assert.interrupted();
    }
}

#[test]
fn test_until_time_only() {
    let mut cmd = Command::cargo_bin("asleep").unwrap();
    cmd.args(["--until", "23:59:59"]).arg("--no-progress");
    assert_interrupted(cmd, Duration::from_secs(3));
}

#[test]
fn test_until_tomorrow() {
    let mut cmd = Command::cargo_bin("asleep").unwrap();
    cmd.args(["--until", "tomorrow 10:00"]).arg("--no-progress");
    assert_interrupted(cmd, Duration::from_secs(3));
}

#[test]
fn test_until_full_date() {
    let mut cmd = Command::cargo_bin("asleep").unwrap();
    cmd.args(["--until", "2099-01-01 12:00:00"])
        .arg("--no-progress");
    assert_interrupted(cmd, Duration::from_secs(3));
}

#[test]
fn test_no_progress_flag() {
    let mut cmd = Command::cargo_bin("asleep").unwrap();
    cmd.args(["1s", "--no-progress"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_monotonic_flag() {
    let mut cmd = Command::cargo_bin("asleep").unwrap();
    cmd.args(["1s", "--monotonic", "--no-progress"])
        .assert()
        .success();
}

#[test]
fn test_short_flags() {
    let mut cmd = Command::cargo_bin("asleep").unwrap();
    cmd.args(["1s", "-n", "-m"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_until_short_flag() {
    let mut cmd = Command::cargo_bin("asleep").unwrap();
    let future = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 1;

    cmd.args(["-u", &format!("@{}", future)])
        .arg("-n")
        .assert()
        .success();
}

#[test]
#[cfg(unix)]
fn test_signal_handling() {
    use nix::sys::signal::{self, Signal};
    use nix::unistd::Pid;
    use std::process::Stdio;
    use std::thread;

    let mut cmd = std::process::Command::new(assert_cmd::cargo::cargo_bin("asleep"));
    cmd.arg("10s").stdout(Stdio::null()).stderr(Stdio::null());

    let mut child = cmd.spawn().expect("Failed to spawn asleep");

    thread::sleep(Duration::from_millis(200));

    signal::kill(Pid::from_raw(child.id() as i32), Signal::SIGINT).expect("Failed to send SIGINT");

    let status = child.wait().expect("Failed to wait for child");
    assert_eq!(status.code(), Some(130));
}
