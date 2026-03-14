use std::path::{Path, Component};
fn main() {
    let p = Path::new("foo/..%2f..%2fetc/passwd");
    for c in p.components() {
        println!("{:?}", c);
    }
}
