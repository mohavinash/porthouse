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

// ---- Edge case tests ----

/// Empty entries list should produce no conflicts.
#[test]
fn test_detect_conflicts_empty_entries() {
    let entries: Vec<PortEntry> = vec![];
    let conflicts = detect_conflicts(&entries);
    assert!(conflicts.is_empty(), "Empty entries should produce no conflicts");
}

/// All entries on the same port with the same PID = not a conflict.
#[test]
fn test_same_port_same_pid_no_conflict() {
    let entries = vec![
        make_entry_addr(8080, 500, "nginx", "0.0.0.0"),
        make_entry_addr(8080, 500, "nginx", "::"),
        make_entry_addr(8080, 500, "nginx", "127.0.0.1"),
    ];
    let conflicts = detect_conflicts(&entries);
    assert!(
        conflicts.is_empty(),
        "Same PID on same port should not be a conflict, got {} conflicts",
        conflicts.len()
    );
}

/// Three-way conflict: 3 different PIDs on the same port.
#[test]
fn test_three_way_conflict() {
    let entries = vec![
        make_entry(9000, 100, "app1"),
        make_entry(9000, 200, "app2"),
        make_entry(9000, 300, "app3"),
    ];
    let conflicts = detect_conflicts(&entries);
    assert_eq!(conflicts.len(), 1, "Should detect one conflict");
    assert_eq!(conflicts[0].entries.len(), 3, "Conflict should have 3 entries");
}

/// suggest_resolution when the conflicted port is 65535 (max u16).
/// Previously this would cause u16 overflow. Now it should return 0.
#[test]
fn test_suggest_resolution_port_65535() {
    let entries = vec![make_entry(65535, 100, "something")];
    let suggestion = suggest_resolution(65535, &entries);
    assert_eq!(
        suggestion, 0,
        "suggest_resolution for port 65535 should return 0 (no higher port available)"
    );
}

/// suggest_resolution when all ports from conflicted_port+1 to 65535 are used.
#[test]
fn test_suggest_resolution_all_ports_taken() {
    // Create entries for ports 8000 through 65535
    let entries: Vec<PortEntry> = (8000u16..=65535)
        .map(|p| make_entry(p, p as u32, "blocker"))
        .collect();
    let suggestion = suggest_resolution(7999, &entries);
    assert_eq!(
        suggestion, 0,
        "When all ports 8000-65535 are taken, suggest_resolution(7999) should return 0"
    );
}

/// suggest_resolution with empty entries should return conflicted_port + 1.
#[test]
fn test_suggest_resolution_empty_entries() {
    let entries: Vec<PortEntry> = vec![];
    let suggestion = suggest_resolution(3000, &entries);
    assert_eq!(
        suggestion, 3001,
        "With no used ports, suggestion should be conflicted_port + 1"
    );
}

/// detect_registry_violations with empty registry should produce no violations.
#[test]
fn test_detect_registry_violations_empty_registry() {
    let registry = Registry::default();
    let entries = vec![make_entry(8000, 100, "node")];
    let violations = detect_registry_violations(&entries, &registry);
    assert!(
        violations.is_empty(),
        "Empty registry should produce no violations"
    );
}

/// detect_registry_violations with empty entries should produce no violations.
#[test]
fn test_detect_registry_violations_empty_entries() {
    let mut registry = Registry::default();
    registry.register("myapp", None, vec![8000], None);
    let entries: Vec<PortEntry> = vec![];
    let violations = detect_registry_violations(&entries, &registry);
    assert!(
        violations.is_empty(),
        "Empty entries should produce no violations"
    );
}

/// detect_registry_violations should match case-insensitively.
#[test]
fn test_detect_registry_violations_case_insensitive() {
    let mut registry = Registry::default();
    registry.register("MyApp", None, vec![3000], None);

    // Process "myapp" should match project "MyApp" (case-insensitive)
    let entries = vec![make_entry(3000, 100, "myapp")];
    let violations = detect_registry_violations(&entries, &registry);
    assert!(
        violations.is_empty(),
        "Case-insensitive match should not be a violation"
    );
}

/// detect_registry_violations with substring match in either direction.
#[test]
fn test_detect_registry_violations_substring_match() {
    let mut registry = Registry::default();
    registry.register("node", None, vec![3000], None);

    // "node-server" contains "node" -> no violation
    let entries = vec![make_entry(3000, 100, "node-server")];
    let violations = detect_registry_violations(&entries, &registry);
    assert!(
        violations.is_empty(),
        "Process name containing project name should not be a violation"
    );

    // Reverse: project "node-server" and process "node"
    let mut registry2 = Registry::default();
    registry2.register("node-server", None, vec![3000], None);
    let entries2 = vec![make_entry(3000, 100, "node")];
    let violations2 = detect_registry_violations(&entries2, &registry2);
    assert!(
        violations2.is_empty(),
        "Project name containing process name should not be a violation"
    );
}

/// Conflicts should be sorted by port number.
#[test]
fn test_conflicts_sorted_by_port() {
    let entries = vec![
        make_entry(9000, 100, "a"),
        make_entry(9000, 200, "b"),
        make_entry(3000, 300, "c"),
        make_entry(3000, 400, "d"),
        make_entry(6000, 500, "e"),
        make_entry(6000, 600, "f"),
    ];
    let conflicts = detect_conflicts(&entries);
    assert_eq!(conflicts.len(), 3);
    assert_eq!(conflicts[0].port, 3000);
    assert_eq!(conflicts[1].port, 6000);
    assert_eq!(conflicts[2].port, 9000);
}

/// Single entry on a port should not be a conflict.
#[test]
fn test_single_entry_no_conflict() {
    let entries = vec![make_entry(8080, 100, "nginx")];
    let conflicts = detect_conflicts(&entries);
    assert!(conflicts.is_empty());
}

/// detect_registry_violations when port is in range (not explicit ports list).
#[test]
fn test_detect_registry_violations_via_range() {
    let mut registry = Registry::default();
    registry.register("webapp", None, vec![], Some((3000, 3010)));

    // Process "node" on port 3005 (in range) doesn't match "webapp"
    let entries = vec![make_entry(3005, 100, "node")];
    let violations = detect_registry_violations(&entries, &registry);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].port, 3005);
    assert_eq!(violations[0].expected_project, "webapp");
}
