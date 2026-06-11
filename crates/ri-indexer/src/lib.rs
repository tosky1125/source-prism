#![allow(missing_docs, reason = "Milestone scaffold exposes no public API yet.")]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

mod generation;
mod graph;
pub mod graph_ids;
mod overlay;
mod search_http;
mod search_sync;
mod search_sync_types;
mod symbols;

pub use generation::{
    FileManifestInput, GenerationError, GenerationRecord, GenerationStatus, PgGenerationStore,
};
pub use graph::{GraphIndexOutcome, GraphStoreError, PgGraphStore};
pub use overlay::{
    BaseFileRecord, OverlayEntry, OverlayFileStatus, OverlayMergedFile, merge_overlay,
};
pub use search_http::{OpenSearchClient, OpenSearchError};
pub use search_sync::PgSearchSyncStore;
pub use search_sync_types::{
    DriftReport, RebuildOutcome, SearchSyncError, SearchSyncInput, SearchSyncOperation,
    SearchSyncRecord, SyncOnceOutcome,
};
pub use symbols::{PgSymbolStore, SymbolStoreError};
