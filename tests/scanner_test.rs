use porthouse::scanner::{scan_ports, is_port_free, suggest_free_ports};

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
    assert!(result == true || result == false);
}
