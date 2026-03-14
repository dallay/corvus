use std::path::Path;
use std::fs;

fn main() {
    let workspace = Path::new("test_workspace");
    fs::create_dir_all(workspace).unwrap();
    fs::create_dir_all(workspace.join("%2e%2e")).unwrap();
    fs::write(workspace.join("%2e%2e/secret.txt"), "hidden").unwrap();

    let path_str = "%2e%2e/secret.txt";
    let full_path = workspace.join(path_str);
    println!("Full path: {:?}", full_path);

    match full_path.canonicalize() {
        Ok(p) => println!("Canonical: {:?}", p),
        Err(e) => println!("Error: {}", e),
    }

    fs::remove_dir_all(workspace).unwrap();
}
