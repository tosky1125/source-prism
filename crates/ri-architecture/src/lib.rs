#![allow(
    missing_docs,
    reason = "Milestone architecture evidence contracts are self-describing."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx and Tree-sitter dependency graph pulls duplicate transitive crates outside this crate's control."
)]

use std::{fs, path::Path};

use ri_core::{CommitSha, EntityId, FilePath, RepoId};
use ri_git::LocalManifest;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ArchitectureError {
    #[error("invalid architecture file path: {path}")]
    InvalidPath {
        path: String,
        source: ri_core::CoreError,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ArchitectureEntityKind {
    Codeowners,
    Adr,
    Documentation,
    OpenApi,
    Graphql,
    DbMigration,
}

impl ArchitectureEntityKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Codeowners => "codeowners",
            Self::Adr => "adr",
            Self::Documentation => "documentation",
            Self::OpenApi => "openapi",
            Self::Graphql => "graphql",
            Self::DbMigration => "db_migration",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ArchitectureEntity {
    pub stable_entity_id: EntityId,
    pub entity_id: EntityId,
    pub kind: ArchitectureEntityKind,
    pub name: String,
    pub file_path: FilePath,
    pub start_line: u32,
    pub end_line: u32,
    pub content_hash: String,
}

impl ArchitectureEntity {
    pub fn new(repo: &RepoId, commit: &CommitSha, spec: ArchitectureEntitySpec) -> Self {
        let kind = spec.kind;
        let name = spec.name;
        let file_path = spec.file_path;
        let content_hash = spec.content_hash;
        Self {
            stable_entity_id: EntityId::stable(repo, &file_path, kind.as_str(), &name),
            entity_id: EntityId::versioned(
                repo,
                commit,
                &file_path,
                kind.as_str(),
                &name,
                &content_hash,
            ),
            kind,
            name,
            file_path,
            start_line: spec.start_line,
            end_line: spec.end_line,
            content_hash,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ArchitectureEntitySpec {
    pub kind: ArchitectureEntityKind,
    pub name: String,
    pub file_path: FilePath,
    pub start_line: u32,
    pub end_line: u32,
    pub content_hash: String,
}

impl ArchitectureEntitySpec {
    pub fn new(
        kind: ArchitectureEntityKind,
        name: impl Into<String>,
        file_path: FilePath,
        start_line: u32,
        end_line: u32,
        content_hash: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            name: name.into(),
            file_path,
            start_line,
            end_line,
            content_hash: content_hash.into(),
        }
    }
}

pub fn extract_architecture_entities_for(
    repo_path: &Path,
    repo: &RepoId,
    commit: &CommitSha,
    manifest: &LocalManifest,
) -> Result<Vec<ArchitectureEntity>, ArchitectureError> {
    let mut entities = Vec::new();
    for file in manifest.files() {
        let Some(kind) = classify_architecture_path(file.path()) else {
            continue;
        };
        let file_path =
            FilePath::new(file.path()).map_err(|source| ArchitectureError::InvalidPath {
                path: file.path().to_owned(),
                source,
            })?;
        let name = entity_name(file.path());
        let end_line = line_count(repo_path.join(file.path()))?;
        let entity = ArchitectureEntity::new(
            repo,
            commit,
            ArchitectureEntitySpec::new(kind, name, file_path, 1, end_line, file.content_sha256()),
        );
        entities.push(entity);
    }
    entities.sort_by(|left, right| {
        left.file_path
            .as_str()
            .cmp(right.file_path.as_str())
            .then_with(|| left.kind.as_str().cmp(right.kind.as_str()))
    });
    Ok(entities)
}

fn classify_architecture_path(path: &str) -> Option<ArchitectureEntityKind> {
    let lower = path.to_ascii_lowercase();
    match lower.as_str() {
        "codeowners" | ".github/codeowners" => Some(ArchitectureEntityKind::Codeowners),
        "openapi.yaml" | "openapi.yml" | "openapi.json" | "swagger.yaml" | "swagger.yml"
        | "swagger.json" => Some(ArchitectureEntityKind::OpenApi),
        "schema.graphql" => Some(ArchitectureEntityKind::Graphql),
        _ if has_suffix(&lower, ".openapi.yaml")
            || has_suffix(&lower, ".openapi.yml")
            || has_suffix(&lower, ".openapi.json") =>
        {
            Some(ArchitectureEntityKind::OpenApi)
        }
        _ if has_extension(&lower, "graphql") => Some(ArchitectureEntityKind::Graphql),
        _ if lower.starts_with("migrations/") && has_extension(&lower, "sql") => {
            Some(ArchitectureEntityKind::DbMigration)
        }
        _ if is_adr_path(&lower) => Some(ArchitectureEntityKind::Adr),
        _ if lower.starts_with("docs/") && has_extension(&lower, "md") => {
            Some(ArchitectureEntityKind::Documentation)
        }
        _ => None,
    }
}

fn is_adr_path(path: &str) -> bool {
    has_extension(path, "md")
        && (path.starts_with("docs/adr/")
            || path.starts_with("doc/adr/")
            || path.starts_with("adr/")
            || path.contains("/adr/"))
}

fn has_extension(path: &str, expected: &str) -> bool {
    Path::new(path)
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn has_suffix(path: &str, suffix: &str) -> bool {
    path.get(path.len().saturating_sub(suffix.len())..)
        .is_some_and(|tail| tail.eq_ignore_ascii_case(suffix))
}

fn entity_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .map_or_else(|| path.to_owned(), ToOwned::to_owned)
}

fn line_count(path: impl AsRef<Path>) -> Result<u32, std::io::Error> {
    let body = fs::read_to_string(path)?;
    let count = body.lines().count().max(1);
    Ok(u32::try_from(count).unwrap_or(u32::MAX))
}
