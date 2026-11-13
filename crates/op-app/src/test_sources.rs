//! Production source inventory shared by translation and embedded-font checks.
use std::path::Path;

pub fn all() -> Vec<(String, String)> {
    fn visit(dir: &Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") && path.file_name().unwrap() != "tests.rs" {
                out.push((path.to_string_lossy().replace('\\', "/"), std::fs::read_to_string(path).unwrap()));
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut result = Vec::new();
    visit(&root.join("src"), &mut result);
    visit(&root.join("../op-core/src"), &mut result);
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}
