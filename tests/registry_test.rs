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
