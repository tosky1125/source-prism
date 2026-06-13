#![allow(
    clippy::redundant_pub_crate,
    reason = "These helpers are private to ri-git but consumed by the parent module."
)]

use std::path::Path;

use ri_core::Language;

pub(crate) fn guess_language(path: &str) -> Language {
    match Path::new(path).extension().and_then(|value| value.to_str()) {
        Some("rs") => Language::Rust,
        Some("ts" | "tsx") => Language::TypeScript,
        Some("js" | "jsx" | "mjs" | "cjs") => Language::JavaScript,
        Some("php") => Language::Php,
        Some("py") => Language::Python,
        Some("java") => Language::Java,
        Some("go") => Language::Go,
        Some(_) | None => Language::Unknown,
    }
}

pub(crate) fn is_generated_path(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    lowered.contains("/generated/")
        || lowered.contains("/gen/")
        || lowered.contains("/assets/repo-explorer/")
        || lowered.starts_with("generated/")
        || lowered.starts_with("gen/")
        || lowered.starts_with("assets/repo-explorer/")
        || lowered.contains(".generated.")
        || lowered.ends_with(".pb.go")
}

pub(crate) fn is_vendor_path(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    lowered == "vendor"
        || lowered.starts_with("vendor/")
        || lowered.contains("/vendor/")
        || lowered == "node_modules"
        || lowered.starts_with("node_modules/")
        || lowered.contains("/node_modules/")
        || lowered == "third_party"
        || lowered.starts_with("third_party/")
        || lowered.contains("/third_party/")
}

pub(crate) fn is_test_path(path: &str) -> bool {
    let lowered = path.to_ascii_lowercase();
    lowered.starts_with("tests/")
        || lowered.contains("/tests/")
        || lowered.contains("_test.")
        || lowered.contains(".test.")
        || lowered.contains(".spec.")
}
