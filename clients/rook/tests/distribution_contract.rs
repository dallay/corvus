use rook::distribution::{current_platform_binary_name, supported_release_artifacts};

#[test]
fn supported_release_artifacts_match_expected_names() {
    let names: Vec<&str> = supported_release_artifacts()
        .iter()
        .map(|artifact| artifact.binary_name)
        .collect();

    assert_eq!(
        names,
        vec![
            "rook-darwin-arm64",
            "rook-darwin-x64",
            "rook-linux-x64",
            "rook-linux-arm64",
            "rook-windows-x64.exe",
        ]
    );
}

#[test]
fn current_platform_binary_name_uses_supported_contract() {
    let binary_name = current_platform_binary_name().expect("current platform should be supported");
    assert!(supported_release_artifacts()
        .iter()
        .any(|artifact| artifact.binary_name == binary_name));
}
