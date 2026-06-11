use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{EdgeKind, EvidenceSpan};

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum CoreError {
    #[error("{kind} cannot be empty")]
    EmptyId { kind: &'static str },
}

macro_rules! id_type {
    ($name:ident, $kind:literal) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl AsRef<str>) -> Result<Self, CoreError> {
                let trimmed = value.as_ref().trim();
                if trimmed.is_empty() {
                    return Err(CoreError::EmptyId { kind: $kind });
                }
                Ok(Self(String::from(trimmed)))
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

id_type!(RepoId, "repo_id");
id_type!(CommitSha, "commit_sha");
id_type!(FilePath, "file_path");
id_type!(SymbolId, "symbol_id");
id_type!(EntityId, "entity_id");
id_type!(EdgeId, "edge_id");
id_type!(ChunkId, "chunk_id");
id_type!(JobId, "job_id");
id_type!(GenerationId, "generation_id");

impl SymbolId {
    pub fn stable(repo: &RepoId, file: &FilePath, fqn: &str) -> Self {
        Self(prefixed_digest("sym", &[repo.as_str(), file.as_str(), fqn]))
    }

    pub fn versioned(
        repo: &RepoId,
        commit: &CommitSha,
        file: &FilePath,
        fqn: &str,
        content_hash: &str,
    ) -> Self {
        Self(prefixed_digest(
            "symv",
            &[
                repo.as_str(),
                commit.as_str(),
                file.as_str(),
                fqn,
                content_hash,
            ],
        ))
    }
}

impl EdgeId {
    pub fn deterministic(
        repo: &RepoId,
        commit: &CommitSha,
        source: &SymbolId,
        target: &SymbolId,
        kind: EdgeKind,
        evidence: &EvidenceSpan,
    ) -> Self {
        let start_line = evidence.start.line.to_string();
        let start_column = evidence.start.column.to_string();
        let end_line = evidence.end.line.to_string();
        let end_column = evidence.end.column.to_string();
        Self(prefixed_digest(
            "edge",
            &[
                repo.as_str(),
                commit.as_str(),
                source.as_str(),
                target.as_str(),
                kind.as_id_part(),
                evidence.file_path.as_str(),
                start_line.as_str(),
                start_column.as_str(),
                end_line.as_str(),
                end_column.as_str(),
            ],
        ))
    }
}

fn prefixed_digest(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hash_part(&mut hasher, prefix);
    for part in parts {
        hash_part(&mut hasher, part);
    }
    format!("{prefix}:{}", hex::encode(hasher.finalize()))
}

fn hash_part(hasher: &mut Sha256, part: &str) {
    let len = part.len().to_string();
    hasher.update(len.as_bytes());
    hasher.update(b":");
    hasher.update(part.as_bytes());
    hasher.update(b";");
}
