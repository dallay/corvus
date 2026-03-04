use std::fs;
use std::path::PathBuf;

// Validation for this file update: ran `cargo test --test legacy_loop_guard`; skipped
// `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`, and full
// `cargo test` because this patch only adjusts path resolution and assertions.

fn runtime_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

#[test]
fn no_legacy_loop_reexport_in_agent_mod() {
    let mod_rs = fs::read_to_string(runtime_path("src/agent/mod.rs")).expect("read agent/mod.rs");
    assert!(
        !mod_rs.contains("pub use loop_")
            && !mod_rs.contains("pub(crate) use loop_")
            && !mod_rs.contains("pub use loop_::"),
        "legacy loop re-export must be removed"
    );
}

#[test]
fn runtime_entrypoints_do_not_reference_loop_module_directly() {
    for path in ["src/main.rs", "src/daemon/mod.rs", "src/cron/scheduler.rs"] {
        let content = fs::read_to_string(runtime_path(path)).expect("read runtime entrypoint");
        assert!(
            !(content.contains("agent::loop_")
                || (content.contains("agent::{") && content.contains("loop_"))),
            "{path} should not call legacy loop module directly"
        );
    }
}

#[test]
fn channels_runtime_has_no_legacy_loop_import() {
    let content =
        fs::read_to_string(runtime_path("src/channels/mod.rs")).expect("read channels/mod.rs");
    assert!(
        !(content.contains("agent::loop_")
            || (content.contains("agent::{") && content.contains("loop_"))),
        "channels runtime should not import legacy loop module"
    );
}

#[test]
fn legacy_loop_file_is_removed() {
    let exists = runtime_path("src/agent/loop_.rs").exists();
    assert!(!exists, "legacy loop file must be removed");
}
