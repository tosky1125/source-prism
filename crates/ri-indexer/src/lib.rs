#![allow(missing_docs, reason = "Milestone scaffold exposes no public API yet.")]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

mod architecture;
mod coverage;
mod generation;
mod generation_files;
mod graph;
mod graph_calls;
pub mod graph_ids;
mod graph_import_paths;
mod graph_imports;
mod graph_query;
mod graph_test_covers;
mod overlay;
mod search_chunks;
mod search_http;
mod search_sync;
mod search_sync_types;
mod symbols;
mod test_cases;
mod test_runs;

pub const DEFAULT_SEARCH_INDEX: &str = "source-prism-dev";

pub use architecture::{ArchitectureEntityRecord, ArchitectureStoreError, PgArchitectureStore};
pub use coverage::{
    CoverageIngestOutcome, CoverageSegmentRecord, CoverageStoreError, PgCoverageStore,
};
pub use generation::{
    FileManifestInput, GenerationError, GenerationRecord, GenerationStatus, PgGenerationStore,
};
pub use generation_files::FileManifestRecord;
pub use graph::{GraphIndexOutcome, GraphStoreError, PgGraphStore};
pub use graph_calls::CallEdgeInput;
pub use graph_query::{GraphEdgeRecord, GraphNodeRecord, GraphProjection};
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
pub use test_cases::{PgTestCaseStore, TestCaseRecord, TestCaseStoreError};
pub use test_runs::{
    PgTestRunStore, TestResultRecord, TestRunIngestOutcome, TestRunRecord, TestRunStoreError,
};
