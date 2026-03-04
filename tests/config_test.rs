#[allow(unused_imports)]
use porthouse::config::{PorthouseConfig, DaemonConfig, AlertConfig, DefaultsConfig};
use tempfile::TempDir;

#[test]
fn test_default_config_has_sane_values() {
    let config = PorthouseConfig::default();
    assert_eq!(config.daemon.scan_interval_secs, 3);
    assert_eq!(config.daemon.port_range, (1024, 49151));
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

// ---- Edge case tests ----

/// Loading a malformed TOML file should return an error.
#[test]
fn test_load_malformed_toml() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad.toml");
    std::fs::write(&path, "this is not valid toml {{{{").unwrap();
    let result = PorthouseConfig::load(&path);
    assert!(result.is_err(), "Malformed TOML should produce an error");
}

/// load_or_default with malformed TOML should fall back to defaults gracefully.
#[test]
fn test_load_or_default_malformed_toml_returns_default() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad.toml");
    std::fs::write(&path, "not valid toml!!! [[[[").unwrap();
    let config = PorthouseConfig::load_or_default(&path);
    assert_eq!(config.daemon.scan_interval_secs, 3, "Should fall back to default");
}

/// A TOML file with missing fields should fail to load (partial config).
#[test]
fn test_load_partial_config_missing_fields() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("partial.toml");
    // Only daemon section, missing alerts and defaults
    std::fs::write(
        &path,
        "[daemon]\nscan_interval_secs = 5\nport_range = [2000, 3000]\n",
    )
    .unwrap();
    let result = PorthouseConfig::load(&path);
    assert!(
        result.is_err(),
        "Partial config missing required sections should fail"
    );
}

/// Saving to a read-only directory should fail.
#[test]
#[cfg(unix)]
fn test_save_to_read_only_path() {
    use std::os::unix::fs::PermissionsExt;
    let dir = TempDir::new().unwrap();
    let readonly_dir = dir.path().join("readonly");
    std::fs::create_dir(&readonly_dir).unwrap();
    std::fs::set_permissions(&readonly_dir, std::fs::Permissions::from_mode(0o444)).unwrap();

    let path = readonly_dir.join("config.toml");
    let config = PorthouseConfig::default();
    let result = config.save(&path);

    // Restore permissions for cleanup
    let _ = std::fs::set_permissions(&readonly_dir, std::fs::Permissions::from_mode(0o755));

    assert!(result.is_err(), "Saving to a read-only path should fail");
}

/// Config with extreme values should roundtrip correctly (scan_interval_secs = 0).
#[test]
fn test_config_extreme_values_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("extreme.toml");

    let config = PorthouseConfig {
        daemon: DaemonConfig {
            scan_interval_secs: 0,
            port_range: (65535, 1024), // inverted range
        },
        alerts: AlertConfig {
            macos_notifications: false,
            terminal_bell: false,
            log_file: String::new(),
            webhook_url: String::new(),
        },
        defaults: DefaultsConfig {
            ports_per_project: 0,
        },
    };
    config.save(&path).unwrap();

    let loaded = PorthouseConfig::load(&path).unwrap();
    assert_eq!(loaded.daemon.scan_interval_secs, 0);
    assert_eq!(loaded.daemon.port_range, (65535, 1024));
    assert_eq!(loaded.defaults.ports_per_project, 0);
}

/// Config with u64::MAX scan_interval_secs cannot be serialized to TOML because
/// TOML integers are limited to i64 range. The save itself fails.
/// This is an important edge case: extremely large u64 values break TOML serialization.
#[test]
fn test_config_max_scan_interval_fails_save() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("maxinterval.toml");

    let config = PorthouseConfig {
        daemon: DaemonConfig {
            scan_interval_secs: u64::MAX,
            port_range: (1024, 65535),
        },
        alerts: AlertConfig {
            macos_notifications: true,
            terminal_bell: true,
            log_file: "~/.porthouse/alerts.log".to_string(),
            webhook_url: String::new(),
        },
        defaults: DefaultsConfig {
            ports_per_project: 10,
        },
    };
    // TOML cannot represent u64::MAX (exceeds i64 range), so save should fail
    let result = config.save(&path);
    assert!(
        result.is_err(),
        "u64::MAX should fail TOML serialization (TOML integers are i64 range)"
    );
}

/// Config with a large but i64-safe value should roundtrip fine.
#[test]
fn test_config_large_but_valid_scan_interval() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("large_interval.toml");

    let config = PorthouseConfig {
        daemon: DaemonConfig {
            scan_interval_secs: i64::MAX as u64, // max safe TOML integer
            port_range: (1024, 65535),
        },
        alerts: AlertConfig {
            macos_notifications: true,
            terminal_bell: true,
            log_file: "~/.porthouse/alerts.log".to_string(),
            webhook_url: String::new(),
        },
        defaults: DefaultsConfig {
            ports_per_project: 10,
        },
    };
    config.save(&path).unwrap();
    let loaded = PorthouseConfig::load(&path).unwrap();
    assert_eq!(loaded.daemon.scan_interval_secs, i64::MAX as u64);
}

/// Complete roundtrip should preserve equality.
#[test]
fn test_config_full_equality_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("eq.toml");
    let config = PorthouseConfig::default();
    config.save(&path).unwrap();
    let loaded = PorthouseConfig::load(&path).unwrap();
    assert_eq!(config, loaded, "Roundtripped config should be equal");
}

/// Loading an empty file should fail.
#[test]
fn test_load_empty_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("empty.toml");
    std::fs::write(&path, "").unwrap();
    let result = PorthouseConfig::load(&path);
    assert!(result.is_err(), "Empty TOML file should fail to deserialize into config");
}

/// Config save should create parent directories if they don't exist.
#[test]
fn test_save_creates_parent_directories() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("a").join("b").join("c").join("config.toml");
    let config = PorthouseConfig::default();
    config.save(&path).unwrap();
    assert!(path.exists(), "Config file should exist after save");
    let loaded = PorthouseConfig::load(&path).unwrap();
    assert_eq!(loaded, config);
}
