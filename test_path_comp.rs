use std::path::Path;
fn main() {
    let paths = vec!["..", "a/../b", "Fixed .. bug", "foo..bar"];
    for p in paths {
        let has_parent = Path::new(p).components().any(|c| matches!(c, std::path::Component::ParentDir));
        println!("Path: '{}', Has ParentDir: {}", p, has_parent);
    }
}
