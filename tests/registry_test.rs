#[allow(unused_imports)]
use porthouse::registry::{Registry, Project};
use tempfile::TempDir;

#[test]
fn test_empty_registry() {
    let registry = Registry::default();
    assert!(registry.projects.is_empty());
}

#[test]
fn test_register_project() {
    let mut registry = Registry::default();
    registry.register("myapp", Some("/path/to/myapp"), vec![3000, 3001], Some((3000, 3010)));
    assert_eq!(registry.projects.len(), 1);
    assert_eq!(registry.projects[0].name, "myapp");
    assert_eq!(registry.projects[0].ports, vec![3000, 3001]);
}

#[test]
fn test_registry_roundtrip_toml() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("registry.toml");

    let mut registry = Registry::default();
    registry.register("app1", Some("/path/app1"), vec![3000], Some((3000, 3010)));
    registry.register("app2", Some("/path/app2"), vec![8000], Some((8000, 8010)));
    registry.save(&path).unwrap();

    let loaded = Registry::load(&path).unwrap();
    assert_eq!(loaded.projects.len(), 2);
    assert_eq!(loaded.projects[0].name, "app1");
    assert_eq!(loaded.projects[1].name, "app2");
}

#[test]
fn test_find_project_by_name() {
    let mut registry = Registry::default();
    registry.register("myapp", Some("/path"), vec![3000], None);
    let found = registry.find_by_name("myapp");
    assert!(found.is_some());
    assert_eq!(found.unwrap().name, "myapp");
    assert!(registry.find_by_name("nonexistent").is_none());
}

#[test]
fn test_find_project_by_port() {
    let mut registry = Registry::default();
    registry.register("app1", None, vec![3000, 3001], None);
    registry.register("app2", None, vec![8000], Some((8000, 8010)));

    assert_eq!(registry.find_by_port(3000).unwrap().name, "app1");
    assert_eq!(registry.find_by_port(8005).unwrap().name, "app2"); // within range
    assert!(registry.find_by_port(9999).is_none());
}

#[test]
fn test_is_port_reserved() {
    let mut registry = Registry::default();
    registry.register("app1", None, vec![3000], Some((3000, 3010)));
    assert!(registry.is_port_reserved(3000));
    assert!(registry.is_port_reserved(3005)); // in range
    assert!(!registry.is_port_reserved(4000));
}

// ---- Edge case tests ----

/// Registering duplicate project names should add both entries.
/// (The current design allows duplicates - both are stored.)
#[test]
fn test_register_duplicate_project_names() {
    let mut registry = Registry::default();
    registry.register("myapp", None, vec![3000], None);
    registry.register("myapp", None, vec![4000], None);
    assert_eq!(registry.projects.len(), 2, "Both duplicate entries should be stored");
    // find_by_name returns the first one
    let found = registry.find_by_name("myapp").unwrap();
    assert_eq!(found.ports, vec![3000], "find_by_name should return the first match");
}

/// Registering projects with overlapping port ranges.
#[test]
fn test_register_overlapping_port_ranges() {
    let mut registry = Registry::default();
    registry.register("app1", None, vec![], Some((3000, 3010)));
    registry.register("app2", None, vec![], Some((3005, 3015)));
    // Port 3007 is in both ranges, find_by_port returns the first match
    let found = registry.find_by_port(3007).unwrap();
    assert_eq!(found.name, "app1", "Overlapping range should return first match");
    // Port 3012 is only in app2's range
    let found2 = registry.find_by_port(3012).unwrap();
    assert_eq!(found2.name, "app2");
}

/// Registering a project with an empty name should succeed (no validation in register).
#[test]
fn test_register_empty_project_name() {
    let mut registry = Registry::default();
    registry.register("", None, vec![5000], None);
    assert_eq!(registry.projects.len(), 1);
    assert_eq!(registry.projects[0].name, "");
    // Should be findable by empty name
    assert!(registry.find_by_name("").is_some());
}

/// Port 65535 (max u16) should be handled correctly.
#[test]
fn test_register_max_port_number() {
    let mut registry = Registry::default();
    registry.register("edge", None, vec![65535], Some((65534, 65535)));
    assert!(registry.is_port_reserved(65535));
    assert!(registry.is_port_reserved(65534));
    assert!(!registry.is_port_reserved(65533));
}

/// Port 0 should be searchable.
#[test]
fn test_find_by_port_zero() {
    let mut registry = Registry::default();
    registry.register("zero-app", None, vec![0], None);
    assert!(registry.is_port_reserved(0));
    assert_eq!(registry.find_by_port(0).unwrap().name, "zero-app");
}

/// Range where lo > hi should not match any ports.
#[test]
fn test_register_inverted_range() {
    let mut registry = Registry::default();
    registry.register("inverted", None, vec![], Some((5000, 3000)));
    // Port 4000 is between 3000 and 5000 numerically but the check is lo <= port <= hi
    // With lo=5000, hi=3000: port >= 5000 && port <= 3000 is always false
    assert!(!registry.is_port_reserved(4000));
    assert!(!registry.is_port_reserved(5000));
    assert!(!registry.is_port_reserved(3000));
}

/// Multiple projects claiming the same explicit port via ports vec.
#[test]
fn test_multiple_projects_same_port() {
    let mut registry = Registry::default();
    registry.register("web", None, vec![8080], None);
    registry.register("api", None, vec![8080], None);
    // find_by_port returns the first match
    let found = registry.find_by_port(8080).unwrap();
    assert_eq!(found.name, "web");
}

/// Roundtrip with an empty projects list.
#[test]
fn test_roundtrip_empty_registry() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("empty_reg.toml");

    let registry = Registry::default();
    registry.save(&path).unwrap();

    let loaded = Registry::load(&path).unwrap();
    assert!(loaded.projects.is_empty());
}

/// Special characters in project name should survive roundtrip.
#[test]
fn test_special_characters_in_project_name() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("special.toml");

    let mut registry = Registry::default();
    registry.register("my-app/v2 (beta)", Some("/path/to/my app"), vec![3000], None);
    registry.register("emoji_project", None, vec![4000], None);
    registry.register("quotes\"and'stuff", None, vec![5000], None);
    registry.save(&path).unwrap();

    let loaded = Registry::load(&path).unwrap();
    assert_eq!(loaded.projects.len(), 3);
    assert_eq!(loaded.projects[0].name, "my-app/v2 (beta)");
    assert_eq!(
        loaded.projects[0].path.as_deref(),
        Some("/path/to/my app")
    );
    assert_eq!(loaded.projects[2].name, "quotes\"and'stuff");
}

/// A project with no ports and no range.
#[test]
fn test_register_project_no_ports_no_range() {
    let mut registry = Registry::default();
    registry.register("bare", None, vec![], None);
    assert_eq!(registry.projects.len(), 1);
    assert!(registry.projects[0].ports.is_empty());
    assert!(registry.projects[0].range.is_none());
    // Should not be found by any port
    assert!(registry.find_by_port(1).is_none());
    assert!(registry.find_by_port(0).is_none());
}

/// Saving and loading should preserve the order of ports.
#[test]
fn test_ports_order_preserved_on_roundtrip() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("order.toml");

    let mut registry = Registry::default();
    registry.register("ordered", None, vec![9000, 3000, 6000, 1000], None);
    registry.save(&path).unwrap();

    let loaded = Registry::load(&path).unwrap();
    assert_eq!(loaded.projects[0].ports, vec![9000, 3000, 6000, 1000]);
}

/// Loading a malformed registry TOML should error.
#[test]
fn test_load_malformed_registry() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad_reg.toml");
    std::fs::write(&path, "[[project]]\nname = ").unwrap();
    let result = Registry::load(&path);
    assert!(result.is_err());
}

/// load_or_default with a bad file should return an empty registry.
#[test]
fn test_load_or_default_bad_file() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad.toml");
    std::fs::write(&path, "{{invalid}}").unwrap();
    let registry = Registry::load_or_default(&path);
    assert!(registry.projects.is_empty());
}
