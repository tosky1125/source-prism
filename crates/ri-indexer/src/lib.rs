#![allow(missing_docs, reason = "Milestone scaffold exposes no public API yet.")]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

mod generation;
mod overlay;

pub use generation::{
    FileManifestInput, GenerationError, GenerationRecord, GenerationStatus, PgGenerationStore,
};
pub use overlay::{
    BaseFileRecord, OverlayEntry, OverlayFileStatus, OverlayMergedFile, merge_overlay,
};
