use std::path::{Path, PathBuf};

pub fn is_path_allowed(path: &str) -> bool {
    if path.contains('\0') {
        return false;
    }
    if Path::new(path)
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return false;
    }
    let lower = path.to_lowercase();
    if lower.contains("..%2f") || lower.contains("%2f..") {
        return false;
    }
    true
}

fn main() {
    let paths = vec![
        "../../etc/passwd",
        "..%2fetc/passwd",
        "%2e%2e%2fetc/passwd",
        "%2e%2e/etc/passwd",
    ];
    for p in paths {
        println!("Path: {}, Allowed: {}", p, is_path_allowed(p));
    }
}
