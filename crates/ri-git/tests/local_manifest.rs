#![allow(
    missing_docs,
    reason = "Integration tests are executable contract names."
)]

use std::fs;
use std::path::Path;

use ri_core::Language;
use ri_git::{FileManifest, LocalManifest};

#[test]
fn local_manifest_is_empty_when_repo_has_no_files() -> Result<(), Box<dyn std::error::Error>> {
    let repo = fixture_repo()?;

    let manifest = LocalManifest::extract(repo.path())?;

    assert!(manifest.files().is_empty());
    Ok(())
}

#[test]
fn local_manifest_omits_deleted_worktree_files() -> Result<(), Box<dyn std::error::Error>> {
    let repo = fixture_repo()?;
    write_file(repo.path(), "src/main.rs", b"fn main() {}\n")?;
    fs::remove_file(repo.path().join("src/main.rs"))?;

    let manifest = LocalManifest::extract(repo.path())?;

    assert!(manifest.files().is_empty());
    Ok(())
}

#[test]
fn local_manifest_skips_binary_files() -> Result<(), Box<dyn std::error::Error>> {
    let repo = fixture_repo()?;
    write_file(repo.path(), "assets/logo.bin", b"source\0prism")?;

    let manifest = LocalManifest::extract(repo.path())?;

    assert!(manifest.files().is_empty());
    Ok(())
}

#[test]
fn local_manifest_skips_local_omo_runtime_artifacts() -> Result<(), Box<dyn std::error::Error>> {
    let repo = fixture_repo()?;
    write_file(
        repo.path(),
        ".omo/ulw-loop/session/ledger.jsonl",
        b"{\"event\":\"x\"}\n",
    )?;
    write_file(repo.path(), "src/lib.rs", b"pub fn real() {}\n")?;

    let manifest = LocalManifest::extract(repo.path())?;

    assert_eq!(manifest.files().len(), 1);
    assert!(find_manifest_file(manifest.files(), "src/lib.rs").is_some());
    assert!(find_manifest_file(manifest.files(), ".omo/ulw-loop/session/ledger.jsonl").is_none());
    Ok(())
}

#[test]
fn local_manifest_detects_generated_vendor_and_test_files() -> Result<(), Box<dyn std::error::Error>>
{
    let repo = fixture_repo()?;
    write_file(
        repo.path(),
        "src/generated/schema.rs",
        b"pub struct Schema;\n",
    )?;
    write_file(
        repo.path(),
        "crates/ri-api/assets/repo-explorer/assets/repo-explorer.js",
        b"export const bundle = 1;\n",
    )?;
    write_file(
        repo.path(),
        "vendor/pkg/index.js",
        b"export const pkg = 1;\n",
    )?;
    write_file(
        repo.path(),
        "tests/contracts.rs",
        b"#[test]\nfn contract() {}\n",
    )?;

    let manifest = LocalManifest::extract(repo.path())?;
    let files = manifest.files();

    assert_eq!(files.len(), 4);
    assert!(
        files
            .iter()
            .any(|file| file.path() == "src/generated/schema.rs"
                && file.is_generated()
                && file.language() == Language::Rust)
    );
    assert!(files.iter().any(|file| file.path()
        == "crates/ri-api/assets/repo-explorer/assets/repo-explorer.js"
        && file.is_generated()
        && file.language() == Language::JavaScript));
    assert!(
        files
            .iter()
            .any(|file| file.path() == "vendor/pkg/index.js" && file.is_vendor())
    );
    assert!(
        files
            .iter()
            .any(|file| file.path() == "tests/contracts.rs" && file.is_test())
    );
    Ok(())
}

#[test]
fn local_manifest_hashes_are_deterministic_for_same_content()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = fixture_repo()?;
    write_file(repo.path(), "src/lib.rs", b"alpha\n")?;

    let first = LocalManifest::extract(repo.path())?;
    let second = LocalManifest::extract(repo.path())?;
    let first_file = find_manifest_file(first.files(), "src/lib.rs");
    let second_file = find_manifest_file(second.files(), "src/lib.rs");

    assert_eq!(
        first_file.map(FileManifest::content_sha256),
        second_file.map(FileManifest::content_sha256)
    );
    assert_eq!(
        first_file.map(FileManifest::content_sha256),
        Some("b6a98d9ce9a2d9149288fa3df42d377c3e42737afdcdaf714e33c0a100b51060")
    );
    assert_eq!(first_file.map(FileManifest::size_bytes), Some(6));
    Ok(())
}

fn fixture_repo() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    gix::init(dir.path())?;
    Ok(dir)
}

fn write_file(
    repo: &Path,
    relative_path: &str,
    content: &[u8],
) -> Result<(), Box<dyn std::error::Error>> {
    let path = repo.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

fn find_manifest_file<'a>(files: &'a [FileManifest], path: &str) -> Option<&'a FileManifest> {
    files.iter().find(|file| file.path() == path)
}
