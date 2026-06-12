#![allow(
    clippy::redundant_pub_crate,
    reason = "Sibling graph modules consume this private-module path resolver."
)]

use std::collections::BTreeSet;

pub(crate) fn resolve_rust_module_file(
    files: &BTreeSet<String>,
    source_file_path: &str,
    module_name: &str,
) -> Option<String> {
    candidate_module_files(source_file_path, module_name)
        .into_iter()
        .find(|candidate| files.contains(candidate))
}

fn candidate_module_files(source_file_path: &str, module_name: &str) -> Vec<String> {
    let Some((dir, stem)) = split_dir_stem(source_file_path) else {
        return Vec::new();
    };
    let base = if matches!(stem, "lib" | "main" | "mod") {
        dir
    } else if dir.is_empty() {
        stem.to_owned()
    } else {
        format!("{dir}/{stem}")
    };
    if base.is_empty() {
        vec![format!("{module_name}.rs"), format!("{module_name}/mod.rs")]
    } else {
        vec![
            format!("{base}/{module_name}.rs"),
            format!("{base}/{module_name}/mod.rs"),
        ]
    }
}

fn split_dir_stem(path: &str) -> Option<(String, &str)> {
    let (dir, file_name) = path.rsplit_once('/').unwrap_or(("", path));
    file_name
        .strip_suffix(".rs")
        .map(|stem| (dir.to_owned(), stem))
}
