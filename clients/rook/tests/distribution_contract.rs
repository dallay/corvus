use rook::distribution::{current_platform_binary_name, supported_release_artifacts};

#[test]
fn supported_release_artifacts_match_expected_names() {
    let artifacts: Vec<(&str, &str)> = supported_release_artifacts()
        .iter()
        .map(|artifact| (artifact.target, artifact.binary_name))
        .collect();

    assert_eq!(
        artifacts,
        vec![
            ("aarch64-apple-darwin", "rook-darwin-arm64"),
            ("x86_64-apple-darwin", "rook-darwin-x64"),
            ("x86_64-unknown-linux-gnu", "rook-linux-x64"),
            ("aarch64-unknown-linux-gnu", "rook-linux-arm64"),
            ("x86_64-pc-windows-msvc", "rook-windows-x64.exe"),
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
