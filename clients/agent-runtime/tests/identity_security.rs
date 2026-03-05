use corvus::identity::load_aieos_identity;
use corvus::config::IdentityConfig;
use tempfile::tempdir;
use std::fs;
use std::os::unix::fs::symlink;

#[test]
fn test_load_aieos_identity_within_workspace() {
    let workspace = tempdir().unwrap();
    let identity_file = workspace.path().join("identity.json");
    fs::write(&identity_file, "{\"identity\":{\"names\":{\"first\":\"Test\"}}}").unwrap();

    let config = IdentityConfig {
        format: "aieos".into(),
        aieos_path: Some("identity.json".into()),
        aieos_inline: None,
    };

    let result = load_aieos_identity(&config, workspace.path()).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().identity.unwrap().names.unwrap().first.unwrap(), "Test");
}

#[test]
fn test_load_aieos_identity_traversal_dots() {
    let workspace = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let identity_file = outside.path().join("identity.json");
    fs::write(&identity_file, "{}").unwrap();

    let relative_path = format!("../{}/identity.json",
        outside.path().file_name().unwrap().to_str().unwrap());

    let config = IdentityConfig {
        format: "aieos".into(),
        aieos_path: Some(relative_path),
        aieos_inline: None,
    };

    let result = load_aieos_identity(&config, workspace.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("outside the workspace"));
}

#[test]
fn test_load_aieos_identity_traversal_symlink() {
    let workspace = tempdir().unwrap();
    let outside = tempdir().unwrap();
    let identity_file = outside.path().join("identity.json");
    fs::write(&identity_file, "{}").unwrap();

    let link_path = workspace.path().join("malicious_link.json");
    symlink(&identity_file, &link_path).unwrap();

    let config = IdentityConfig {
        format: "aieos".into(),
        aieos_path: Some("malicious_link.json".into()),
        aieos_inline: None,
    };

    let result = load_aieos_identity(&config, workspace.path());
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("outside the workspace"));
}
