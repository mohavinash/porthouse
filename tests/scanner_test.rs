use porthouse::scanner::{scan_ports, scan_ports_in_range, is_port_free, suggest_free_ports};

#[test]
fn test_scan_returns_entries() {
    let entries = scan_ports().unwrap();
    // There should be at least one listening port on a running system
    assert!(!entries.is_empty(), "Expected at least one listening port");
}

#[test]
fn test_port_entry_has_required_fields() {
    let entries = scan_ports().unwrap();
    let first = &entries[0];
    assert!(first.port > 0);
    assert!(first.pid > 0);
    assert!(!first.process_name.is_empty());
}

#[test]
fn test_suggest_free_ports_returns_requested_count() {
    let ports = suggest_free_ports(3, (49000, 49100)).unwrap();
    assert_eq!(ports.len(), 3);
    // All should be unique
    let unique: std::collections::HashSet<u16> = ports.iter().cloned().collect();
    assert_eq!(unique.len(), 3);
}

#[test]
fn test_is_port_free_for_unlikely_port() {
    // Port 59999 is unlikely to be in use
    let result = is_port_free(59999).unwrap();
    // We can't guarantee it's free, but the function should work without errors
    // We can't guarantee it's free, but the function should work without errors
    let _ = result; // just verify it doesn't error
}

// ---- Edge case tests ----

/// Requesting 0 free ports should succeed with an empty vec.
#[test]
fn test_suggest_free_ports_zero_count() {
    let ports = suggest_free_ports(0, (49000, 49100)).unwrap();
    assert!(ports.is_empty(), "Requesting 0 ports should return empty vec");
}

/// Requesting more ports than available in a tiny range should fail with a clear error.
#[test]
fn test_suggest_free_ports_more_than_available_in_tiny_range() {
    // Range 65530..=65535 has at most 6 ports; request 100
    let result = suggest_free_ports(100, (65530, 65535));
    assert!(result.is_err(), "Should fail when not enough ports in range");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("free ports"),
        "Error should mention free ports: {}",
        err_msg
    );
}

/// An inverted range (start > end) should produce a clear error.
#[test]
fn test_suggest_free_ports_inverted_range() {
    let result = suggest_free_ports(1, (100, 50));
    assert!(result.is_err(), "Inverted range should fail");
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("Invalid port range") || err_msg.contains("greater than"),
        "Error should indicate inverted range: {}",
        err_msg
    );
}

/// Single-port range scan should succeed.
#[test]
fn test_scan_ports_in_range_single_port() {
    // scan_ports_in_range(65535, 65535) should work without panic
    let result = scan_ports_in_range(65535, 65535);
    assert!(result.is_ok(), "Single port range should not error");
    // Result is either empty or has entries on port 65535
    for entry in result.unwrap() {
        assert_eq!(entry.port, 65535);
    }
}

/// Inverted range scan should return empty (no entries match).
#[test]
fn test_scan_ports_in_range_inverted() {
    let result = scan_ports_in_range(100, 50).unwrap();
    assert!(
        result.is_empty(),
        "Inverted range should return no entries"
    );
}

/// is_port_free(0) should always return true because port 0 entries
/// are filtered out by scan_ports.
#[test]
fn test_is_port_free_port_zero() {
    let result = is_port_free(0).unwrap();
    // Port 0 is not a real port; scanner filters out port 0 entries
    assert!(result, "Port 0 should always appear free (filtered out by scanner)");
}

/// Entries should be sorted by port number.
#[test]
fn test_scan_ports_sorted() {
    let entries = scan_ports().unwrap();
    for i in 1..entries.len() {
        assert!(
            entries[i].port >= entries[i - 1].port,
            "Entries should be sorted by port: {} came after {}",
            entries[i].port,
            entries[i - 1].port
        );
    }
}

/// Suggest free ports should return ports within the requested range.
#[test]
fn test_suggest_free_ports_within_range() {
    let ports = suggest_free_ports(3, (50000, 50100)).unwrap();
    for port in &ports {
        assert!(
            *port >= 50000 && *port <= 50100,
            "Suggested port {} is outside requested range 50000-50100",
            port
        );
    }
}

/// Suggest free ports should return unique ports.
#[test]
fn test_suggest_free_ports_all_unique() {
    let ports = suggest_free_ports(5, (49000, 49200)).unwrap();
    let unique: std::collections::HashSet<u16> = ports.iter().cloned().collect();
    assert_eq!(ports.len(), unique.len(), "Suggested ports should be unique");
}

/// Requesting exactly 1 port from a range that has at least 1 free port.
#[test]
fn test_suggest_free_ports_exactly_one() {
    let ports = suggest_free_ports(1, (60000, 60100)).unwrap();
    assert_eq!(ports.len(), 1);
}
