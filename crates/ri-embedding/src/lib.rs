#![allow(
    missing_docs,
    reason = "Embedding cache contracts are exercised through tests and CLI JSON."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx and TLS dependencies pull duplicate platform crates outside this crate's control."
)]

mod model;
mod store;

pub use model::{
    EmbeddingCacheEntry, EmbeddingCacheError, EmbeddingCacheInput, EmbeddingCacheWrite,
    EmbeddingVector,
};
pub use store::PgEmbeddingCache;
