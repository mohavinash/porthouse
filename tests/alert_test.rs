use porthouse::alert::{AlertEvent, AlertManager};
use porthouse::config::AlertConfig;
use tempfile::TempDir;

#[test]
fn test_alert_event_conflict_formatting() {
    let event = AlertEvent::Conflict {
        port: 8000,
        processes: vec![
            ("flask".to_string(), 100),
            ("uvicorn".to_string(), 200),
        ],
    };
    let msg = event.to_message();
    assert!(msg.contains("8000"));
    assert!(msg.contains("flask"));
    assert!(msg.contains("uvicorn"));
}

#[test]
fn test_alert_event_new_listener_formatting() {
    let event = AlertEvent::NewListener {
        port: 3000,
        process: "node".to_string(),
        pid: 1234,
    };
    let msg = event.to_message();
    assert!(msg.contains("3000"));
    assert!(msg.contains("node"));
}

#[test]
fn test_alert_event_port_freed_formatting() {
    let event = AlertEvent::PortFreed { port: 8080 };
    let msg = event.to_message();
    assert!(msg.contains("8080"));
    assert!(msg.contains("free"));
}

#[test]
fn test_log_file_alert() {
    let dir = TempDir::new().unwrap();
    let log_path = dir.path().join("alerts.log");

    let config = AlertConfig {
        macos_notifications: false,
        terminal_bell: false,
        log_file: log_path.to_string_lossy().to_string(),
        webhook_url: String::new(),
    };

    let manager = AlertManager::new(config);
    let event = AlertEvent::Conflict {
        port: 8000,
        processes: vec![("flask".to_string(), 100)],
    };

    manager.log_to_file(&event).unwrap();
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("8000"));
    assert!(content.contains("flask"));
}

#[test]
fn test_alert_event_titles() {
    let conflict = AlertEvent::Conflict {
        port: 8000,
        processes: vec![],
    };
    assert!(conflict.title().contains("Conflict"));

    let new_listener = AlertEvent::NewListener {
        port: 3000,
        process: "x".into(),
        pid: 1,
    };
    assert!(new_listener.title().contains("Service"));

    let freed = AlertEvent::PortFreed { port: 8080 };
    assert!(freed.title().contains("Freed"));
}

// ---- Edge case tests ----

/// AlertEvent::Conflict with an empty processes list should not panic.
#[test]
fn test_alert_conflict_empty_processes() {
    let event = AlertEvent::Conflict {
        port: 9000,
        processes: vec![],
    };
    let msg = event.to_message();
    assert!(msg.contains("9000"), "Message should contain the port");
}

/// AlertEvent with a very long process name should not panic or truncate.
#[test]
fn test_alert_very_long_process_name() {
    let long_name = "a".repeat(10000);
    let event = AlertEvent::NewListener {
        port: 5000,
        process: long_name.clone(),
        pid: 42,
    };
    let msg = event.to_message();
    assert!(msg.contains(&long_name), "Full long name should appear in message");
    assert!(msg.contains("5000"));
}

/// Log file writing when the parent directory does not exist should create it.
#[test]
fn test_log_to_file_creates_parent_directory() {
    let dir = TempDir::new().unwrap();
    let log_path = dir.path().join("subdir").join("nested").join("alerts.log");

    let config = AlertConfig {
        macos_notifications: false,
        terminal_bell: false,
        log_file: log_path.to_string_lossy().to_string(),
        webhook_url: String::new(),
    };

    let manager = AlertManager::new(config);
    let event = AlertEvent::PortFreed { port: 1234 };
    manager.log_to_file(&event).unwrap();

    assert!(log_path.exists(), "Log file should be created in nested directory");
    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("1234"));
}

/// Multiple log events should append, not overwrite.
#[test]
fn test_log_file_append_behavior() {
    let dir = TempDir::new().unwrap();
    let log_path = dir.path().join("append_test.log");

    let config = AlertConfig {
        macos_notifications: false,
        terminal_bell: false,
        log_file: log_path.to_string_lossy().to_string(),
        webhook_url: String::new(),
    };

    let manager = AlertManager::new(config);

    let event1 = AlertEvent::PortFreed { port: 1111 };
    let event2 = AlertEvent::PortFreed { port: 2222 };
    let event3 = AlertEvent::NewListener {
        port: 3333,
        process: "test".to_string(),
        pid: 999,
    };

    manager.log_to_file(&event1).unwrap();
    manager.log_to_file(&event2).unwrap();
    manager.log_to_file(&event3).unwrap();

    let content = std::fs::read_to_string(&log_path).unwrap();
    assert!(content.contains("1111"), "First event should be in log");
    assert!(content.contains("2222"), "Second event should be in log");
    assert!(content.contains("3333"), "Third event should be in log");

    // Count lines (each event is one line)
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 3, "Should have exactly 3 log lines");
}

/// fire() with all channels disabled should not panic.
#[test]
fn test_fire_all_channels_disabled() {
    let config = AlertConfig {
        macos_notifications: false,
        terminal_bell: false,
        log_file: String::new(), // empty = disabled
        webhook_url: String::new(), // empty = disabled
    };

    let manager = AlertManager::new(config);
    let event = AlertEvent::Conflict {
        port: 8000,
        processes: vec![("a".to_string(), 1), ("b".to_string(), 2)],
    };

    // Should not panic
    manager.fire(&event);
}

/// Log file with timestamp should contain a date-like pattern.
#[test]
fn test_log_file_contains_timestamp() {
    let dir = TempDir::new().unwrap();
    let log_path = dir.path().join("ts_test.log");

    let config = AlertConfig {
        macos_notifications: false,
        terminal_bell: false,
        log_file: log_path.to_string_lossy().to_string(),
        webhook_url: String::new(),
    };

    let manager = AlertManager::new(config);
    let event = AlertEvent::PortFreed { port: 7777 };
    manager.log_to_file(&event).unwrap();

    let content = std::fs::read_to_string(&log_path).unwrap();
    // Timestamp format is [YYYY-MM-DD HH:MM:SS]
    assert!(
        content.contains("[20"),
        "Log line should contain a timestamp starting with [20: {}",
        content
    );
}

/// Conflict event with many processes should format correctly.
#[test]
fn test_alert_conflict_many_processes() {
    let processes: Vec<(String, u32)> = (0..100)
        .map(|i| (format!("proc_{}", i), i))
        .collect();
    let event = AlertEvent::Conflict {
        port: 8080,
        processes,
    };
    let msg = event.to_message();
    assert!(msg.contains("proc_0"));
    assert!(msg.contains("proc_99"));
}

/// PortFreed event formatting.
#[test]
fn test_alert_port_freed_edge_ports() {
    let event0 = AlertEvent::PortFreed { port: 0 };
    assert!(event0.to_message().contains("0"));

    let event_max = AlertEvent::PortFreed { port: 65535 };
    assert!(event_max.to_message().contains("65535"));
}
