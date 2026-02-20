//! Pilot plugin artifact for `memory.surreal.graphs`.
//! The runtime currently validates and installs this plugin as a signed, pinned WASM artifact.

#[no_mangle]
pub extern "C" fn run() -> i32 {
    0
}
