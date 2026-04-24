#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReleaseArtifact {
    pub target: &'static str,
    pub binary_name: &'static str,
}

const SUPPORTED_RELEASE_ARTIFACTS: &[ReleaseArtifact] = &[
    ReleaseArtifact {
        target: "aarch64-apple-darwin",
        binary_name: "rook-darwin-arm64",
    },
    ReleaseArtifact {
        target: "x86_64-apple-darwin",
        binary_name: "rook-darwin-x64",
    },
    ReleaseArtifact {
        target: "x86_64-unknown-linux-gnu",
        binary_name: "rook-linux-x64",
    },
    ReleaseArtifact {
        target: "aarch64-unknown-linux-gnu",
        binary_name: "rook-linux-arm64",
    },
    ReleaseArtifact {
        target: "x86_64-pc-windows-msvc",
        binary_name: "rook-windows-x64.exe",
    },
];

pub fn supported_release_artifacts() -> &'static [ReleaseArtifact] {
    SUPPORTED_RELEASE_ARTIFACTS
}

pub fn current_platform_binary_name() -> Option<&'static str> {
    let target = current_rust_target_triple()?;
    supported_release_artifacts()
        .iter()
        .find(|artifact| artifact.target == target)
        .map(|artifact| artifact.binary_name)
}

fn current_rust_target_triple() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("linux", "x86_64") => Some("x86_64-unknown-linux-gnu"),
        ("linux", "aarch64") => Some("aarch64-unknown-linux-gnu"),
        ("windows", "x86_64") => Some("x86_64-pc-windows-msvc"),
        _ => None,
    }
}
