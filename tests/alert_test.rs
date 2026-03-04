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
    assert!(msg.contains("1234"));
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
    assert!(new_listener.title().contains("Listener"));

    let freed = AlertEvent::PortFreed { port: 8080 };
    assert!(freed.title().contains("Freed"));
}
