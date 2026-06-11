#![allow(
    missing_docs,
    reason = "Core contract names are self-describing at this milestone."
)]

mod evidence;
mod ids;
mod taxonomy;

pub use evidence::{EvidenceSpan, SourcePosition, TrustedInstructions, UntrustedEvidence};
pub use ids::{
    ChunkId, CommitSha, CoreError, EdgeId, EntityId, FilePath, GenerationId, JobId, RepoId,
    SymbolId,
};
pub use taxonomy::{Confidence, EdgeKind, EvidenceSourceKind, Language, SymbolKind, TrustLevel};
