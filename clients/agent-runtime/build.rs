//! Build script for corvus-agent-runtime.
//! Embeds the catalog index snapshot for offline-first skill discovery.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let src = Path::new("src/skills/catalog_index.toml");
    let dst = Path::new(&out_dir).join("catalog_index.toml");

    fs::copy(src, &dst).expect("failed to copy catalog_index.toml to OUT_DIR");

    println!("cargo:rerun-if-changed=src/skills/catalog_index.toml");
}
