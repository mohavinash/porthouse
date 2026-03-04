use porthouse::conflict::{Conflict, detect_conflicts, detect_registry_violations, suggest_resolution};
use porthouse::scanner::PortEntry;
use porthouse::registry::Registry;

fn make_entry(port: u16, pid: u32, name: &str) -> PortEntry {
    PortEntry {
        port,
        pid,
        process_name: name.to_string(),
        protocol: "TCP".to_string(),
        address: "127.0.0.1".to_string(),
    }
}

fn make_entry_addr(port: u16, pid: u32, name: &str, addr: &str) -> PortEntry {
    PortEntry {
        port,
        pid,
        process_name: name.to_string(),
        protocol: "TCP".to_string(),
        address: addr.to_string(),
    }
}

#[test]
fn test_no_conflicts_when_unique_ports() {
    let entries = vec![
        make_entry(3000, 100, "node"),
        make_entry(8000, 200, "python"),
    ];
    let conflicts = detect_conflicts(&entries);
    assert!(conflicts.is_empty());
}

#[test]
fn test_detects_same_port_conflict() {
    let entries = vec![
        make_entry(8000, 100, "flask"),
        make_entry(8000, 200, "uvicorn"),
    ];
    let conflicts = detect_conflicts(&entries);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].port, 8000);
    assert_eq!(conflicts[0].entries.len(), 2);
}

#[test]
fn test_multiple_conflicts() {
    let entries = vec![
        make_entry(3000, 100, "node"),
        make_entry(3000, 101, "deno"),
        make_entry(8000, 200, "flask"),
        make_entry(8000, 201, "uvicorn"),
    ];
    let conflicts = detect_conflicts(&entries);
    assert_eq!(conflicts.len(), 2);
}

#[test]
fn test_suggest_resolution_finds_free_port() {
    let entries = vec![
        make_entry(8000, 100, "flask"),
        make_entry(8000, 200, "uvicorn"),
        make_entry(8001, 300, "something"),
    ];
    let suggestion = suggest_resolution(8000, &entries);
    assert!(suggestion > 8000);
    assert!(!entries.iter().any(|e| e.port == suggestion));
}

#[test]
fn test_suggest_resolution_skips_used_ports() {
    let entries = vec![
        make_entry(8000, 100, "a"),
        make_entry(8001, 200, "b"),
        make_entry(8002, 300, "c"),
    ];
    let suggestion = suggest_resolution(8000, &entries);
    assert_eq!(suggestion, 8003);
}

#[test]
fn test_detect_registry_violations() {
    let mut registry = Registry::default();
    registry.register("MyApp", None, vec![8000], None);

    // "node" does not match "MyApp" in either direction -> violation
    let entries = vec![make_entry(8000, 100, "node")];
    let violations = detect_registry_violations(&entries, &registry);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].port, 8000);
    assert_eq!(violations[0].expected_project, "MyApp");
    assert_eq!(violations[0].actual_process, "node");
}

#[test]
fn test_no_registry_violation_when_name_matches() {
    let mut registry = Registry::default();
    registry.register("node", None, vec![3000], None);

    // "node" matches "node" -> no violation
    let entries = vec![make_entry(3000, 100, "node")];
    let violations = detect_registry_violations(&entries, &registry);
    assert!(violations.is_empty());
}

#[test]
fn test_conflict_struct_fields() {
    let entries = vec![
        make_entry(4000, 10, "alpha"),
        make_entry(4000, 20, "beta"),
    ];
    let conflicts = detect_conflicts(&entries);
    assert_eq!(conflicts.len(), 1);
    let conflict: &Conflict = &conflicts[0];
    assert_eq!(conflict.port, 4000);
    assert_eq!(conflict.entries.len(), 2);
}

#[test]
fn test_same_pid_different_addresses_is_not_a_conflict() {
    // Same process binding to both IPv4 and IPv6 (dual-stack) is normal
    let entries = vec![
        make_entry_addr(5000, 518, "ControlCenter", "0.0.0.0"),
        make_entry_addr(5000, 518, "ControlCenter", "::"),
    ];
    let conflicts = detect_conflicts(&entries);
    assert!(conflicts.is_empty(), "Same PID on same port with different addresses should not be a conflict");
}

#[test]
fn test_different_pids_same_port_is_a_conflict() {
    // postgres (native) and docker postgres on same port = real conflict
    let entries = vec![
        make_entry_addr(5432, 65109, "postgres", "127.0.0.1"),
        make_entry_addr(5432, 30215, "com.docker.backend", "::"),
    ];
    let conflicts = detect_conflicts(&entries);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].port, 5432);
}
