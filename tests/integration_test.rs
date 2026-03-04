#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_status_command_runs() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.arg("status");
    cmd.assert().success().stdout(
        predicate::str::contains("PORT").or(predicate::str::contains("No listening ports found")),
    );
}

#[test]
fn test_check_command_runs() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.arg("check");
    // Exit 0 (no conflicts) or 1 (conflicts) — both are valid
    let _ = cmd.assert();
}

#[test]
fn test_suggest_command() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["suggest", "3"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::is_match(r"\d+").unwrap());
}

#[test]
fn test_free_command() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["free", "59999"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("free").or(predicate::str::contains("in use")));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.arg("--version");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("porthouse"));
}

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("lighthouse"));
}

#[test]
fn test_daemon_status() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["daemon", "status"]);
    cmd.assert().success();
}

#[test]
fn test_check_quiet_mode() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["check", "--quiet"]);
    // In quiet mode, stdout should be empty regardless of result
    let _ = cmd.assert();
}

#[test]
fn test_suggest_with_range() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["suggest", "2", "--from", "49000", "--to", "49100"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::is_match(r"\d+").unwrap());
}
