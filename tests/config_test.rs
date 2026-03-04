#[allow(unused_imports)]
use porthouse::config::{PorthouseConfig, DaemonConfig, AlertConfig, DefaultsConfig};
use tempfile::TempDir;

#[test]
fn test_default_config_has_sane_values() {
    let config = PorthouseConfig::default();
    assert_eq!(config.daemon.scan_interval_secs, 3);
    assert_eq!(config.daemon.port_range, (1024, 65535));
    assert!(config.alerts.macos_notifications);
    assert!(config.alerts.terminal_bell);
    assert_eq!(config.defaults.ports_per_project, 10);
}

#[test]
fn test_config_roundtrip_toml() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.toml");

    let config = PorthouseConfig::default();
    config.save(&path).unwrap();

    let loaded = PorthouseConfig::load(&path).unwrap();
    assert_eq!(loaded.daemon.scan_interval_secs, config.daemon.scan_interval_secs);
    assert_eq!(loaded.alerts.webhook_url, config.alerts.webhook_url);
}

#[test]
fn test_load_missing_config_returns_default() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nonexistent.toml");
    let config = PorthouseConfig::load_or_default(&path);
    assert_eq!(config.daemon.scan_interval_secs, 3);
}
