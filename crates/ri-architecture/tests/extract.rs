#![allow(missing_docs, reason = "Integration test names document behavior.")]

use std::{fs, path::Path, process::Command};

use ri_architecture::{ArchitectureEntityKind, extract_architecture_entities_for};
use ri_core::{CommitSha, RepoId};
use ri_git::LocalManifest;

#[test]
fn extracts_architecture_entities_from_repo_surface() -> Result<(), Box<dyn std::error::Error>> {
    let repo = tempfile::tempdir()?;
    fs::create_dir_all(repo.path().join(".github"))?;
    fs::create_dir_all(repo.path().join("docs/adr"))?;
    fs::create_dir_all(repo.path().join("migrations"))?;
    fs::write(repo.path().join(".github/CODEOWNERS"), "* @platform\n")?;
    fs::write(repo.path().join("docs/adr/0001-records.md"), "# ADR\n")?;
    fs::write(repo.path().join("openapi.yaml"), "openapi: 3.1.0\n")?;
    fs::write(
        repo.path().join("schema.graphql"),
        "type Query { ping: String }\n",
    )?;
    fs::write(
        repo.path().join("migrations/0001_init.sql"),
        "CREATE TABLE invoices(id INT);\n",
    )?;
    run_git(repo.path(), ["init"])?;
    run_git(
        repo.path(),
        ["config", "user.email", "source-prism@example.invalid"],
    )?;
    run_git(repo.path(), ["config", "user.name", "Source Prism Test"])?;
    run_git(repo.path(), ["add", "."])?;
    run_git(repo.path(), ["commit", "-m", "fixture"])?;

    let manifest = LocalManifest::extract(repo.path())?;
    let entities = extract_architecture_entities_for(
        repo.path(),
        &RepoId::new("repo")?,
        &CommitSha::new("commit")?,
        &manifest,
    )?;

    assert!(has_kind(&entities, ArchitectureEntityKind::Codeowners));
    assert!(has_kind(&entities, ArchitectureEntityKind::Adr));
    assert!(has_kind(&entities, ArchitectureEntityKind::OpenApi));
    assert!(has_kind(&entities, ArchitectureEntityKind::Graphql));
    assert!(has_kind(&entities, ArchitectureEntityKind::DbMigration));
    Ok(())
}

fn has_kind(
    entities: &[ri_architecture::ArchitectureEntity],
    kind: ArchitectureEntityKind,
) -> bool {
    entities.iter().any(|entity| entity.kind == kind)
}

fn run_git<const N: usize>(path: &Path, args: [&str; N]) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(path).args(args).output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(std::io::Error::other(String::from_utf8_lossy(&output.stderr).to_string()).into())
}
