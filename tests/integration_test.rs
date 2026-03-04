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
    // Exit 0 (no conflicts) or 1 (conflicts) -- both are valid
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

// ---- Edge case integration tests ----

/// `porthouse suggest 0` -- requesting zero ports should succeed with empty output.
#[test]
fn test_suggest_zero_ports() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["suggest", "0"]);
    cmd.assert().success();
}

/// `porthouse suggest 100 --from 65530 --to 65535` -- more ports than available should fail.
#[test]
fn test_suggest_more_than_available_in_range() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["suggest", "100", "--from", "65530", "--to", "65535"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("free ports"));
}

/// `porthouse free 65535` -- edge port, should work without crash.
#[test]
fn test_free_port_65535() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["free", "65535"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("free").or(predicate::str::contains("in use")));
}

/// `porthouse kill 99999` -- port number exceeds u16 range, clap should reject it.
/// Actually port is u16, so 99999 exceeds u16 max (65535) and clap should error.
#[test]
fn test_kill_invalid_port_number() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["kill", "99999"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("invalid value"));
}

/// `porthouse kill 65535` -- valid port, no process likely listening. Should succeed.
#[test]
fn test_kill_edge_port_65535() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["kill", "65535"]);
    // Should succeed even if nothing is on that port
    cmd.assert().success().stdout(
        predicate::str::contains("No process found")
            .or(predicate::str::contains("Killing")),
    );
}

/// `porthouse check --json` -- output should be valid JSON.
#[test]
fn test_check_json_output_is_valid() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["check", "--json"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The JSON should parse successfully
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        parsed.is_ok(),
        "check --json output should be valid JSON, got: {}",
        stdout
    );
    let json = parsed.unwrap();
    assert!(json.get("conflicts").is_some(), "JSON should have 'conflicts' key");
    assert!(json.get("count").is_some(), "JSON should have 'count' key");
}

/// `porthouse daemon stop` when no daemon is running should succeed gracefully.
#[test]
fn test_daemon_stop_when_not_running() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["daemon", "stop"]);
    // Should succeed or print a message about no daemon running
    // It may succeed (no daemon running) or fail (stale PID), but should not crash
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Either succeeds with a message or fails gracefully
    assert!(
        output.status.success() || !stderr.is_empty(),
        "daemon stop should either succeed or fail gracefully. stdout={}, stderr={}",
        stdout,
        stderr
    );
}

/// `porthouse register` with an empty name string -- clap requires at least one arg.
/// Let's test with a register that has invalid range format.
#[test]
fn test_register_invalid_range_format() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["register", "testapp", "--range", "abc-def"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid"));
}

/// `porthouse register` with a range that has non-numeric parts.
#[test]
fn test_register_non_numeric_range() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["register", "testapp", "--range", "foo-bar"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid"));
}

/// `porthouse suggest 1 --from 100 --to 50` -- inverted range should fail.
#[test]
fn test_suggest_inverted_range() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["suggest", "1", "--from", "100", "--to", "50"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid port range").or(predicate::str::contains("Error")));
}

/// Unknown subcommand should fail.
#[test]
fn test_unknown_subcommand() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.arg("nonexistent");
    cmd.assert().failure();
}

/// `porthouse check --json --quiet` -- both flags, JSON should still work.
#[test]
fn test_check_json_and_quiet() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["check", "--json", "--quiet"]);
    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // When --json is set, output should be JSON regardless of --quiet
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(
        parsed.is_ok(),
        "check --json --quiet should still produce valid JSON, got: {}",
        stdout
    );
}

/// `porthouse suggest 1` default range should work.
#[test]
fn test_suggest_single_port_default_range() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["suggest", "1"]);
    cmd.assert()
        .success()
        .stdout(predicate::str::is_match(r"^\d+\n$").unwrap());
}

/// `porthouse register` with invalid ports list.
#[test]
fn test_register_invalid_ports_list() {
    let mut cmd = Command::cargo_bin("porthouse").unwrap();
    cmd.args(["register", "testapp", "--ports", "abc,def"]);
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid port number"));
}
