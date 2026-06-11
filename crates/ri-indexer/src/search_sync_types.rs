#![allow(
    clippy::redundant_pub_crate,
    reason = "Private module helpers are shared by sibling modules but intentionally not exported."
)]

use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::Row as _;

use crate::OpenSearchError;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SearchSyncError {
    #[error(transparent)]
    OpenSearch(#[from] OpenSearchError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SearchSyncOperation {
    Upsert,
    Delete,
}

impl SearchSyncOperation {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Upsert => "upsert",
            Self::Delete => "delete",
        }
    }

    pub(crate) fn parse(raw: &str) -> Self {
        if raw == "delete" {
            Self::Delete
        } else {
            Self::Upsert
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SearchSyncInput {
    pub repo_id: String,
    pub generation_id: Option<String>,
    pub entity_type: String,
    pub entity_id: String,
    pub operation: SearchSyncOperation,
    pub target_index: String,
    pub payload: Value,
}

impl SearchSyncInput {
    pub fn upsert(
        repo_id: &str,
        entity_type: &str,
        entity_id: &str,
        target_index: &str,
        payload: Value,
    ) -> Self {
        Self {
            repo_id: repo_id.to_owned(),
            generation_id: None,
            entity_type: entity_type.to_owned(),
            entity_id: entity_id.to_owned(),
            operation: SearchSyncOperation::Upsert,
            target_index: target_index.to_owned(),
            payload,
        }
    }

    pub fn upsert_for_generation(
        repo_id: &str,
        generation_id: &str,
        entity_type: &str,
        entity_id: &str,
        target_index: &str,
        payload: Value,
    ) -> Self {
        Self {
            repo_id: repo_id.to_owned(),
            generation_id: Some(generation_id.to_owned()),
            entity_type: entity_type.to_owned(),
            entity_id: entity_id.to_owned(),
            operation: SearchSyncOperation::Upsert,
            target_index: target_index.to_owned(),
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SearchSyncRecord {
    pub outbox_id: String,
    pub entity_id: String,
    pub operation: SearchSyncOperation,
    pub target_index: String,
    pub payload_hash: String,
    pub payload: Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SyncOnceOutcome {
    pub processed: bool,
    pub outbox_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RebuildOutcome {
    pub indexed: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DriftReport {
    pub expected_documents: i64,
    pub actual_documents: i64,
}

impl DriftReport {
    pub const fn has_drift(&self) -> bool {
        self.expected_documents != self.actual_documents
    }
}

pub(crate) fn record_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<SearchSyncRecord, SearchSyncError> {
    Ok(SearchSyncRecord {
        outbox_id: row.try_get("outbox_id")?,
        entity_id: row.try_get("entity_id")?,
        operation: SearchSyncOperation::parse(row.try_get::<String, _>("operation")?.as_str()),
        target_index: row.try_get("target_index")?,
        payload_hash: row.try_get("payload_hash")?,
        payload: row.try_get("payload")?,
    })
}

pub(crate) fn payload_hash(payload: &Value) -> String {
    let encoded = serde_json::to_vec(payload).unwrap_or_else(|_| b"null".to_vec());
    let mut hasher = Sha256::new();
    hasher.update(encoded);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

pub(crate) fn outbox_id(input: &SearchSyncInput, payload_hash: &str) -> String {
    let mut hasher = Sha256::new();
    for part in [
        input.target_index.as_str(),
        input.entity_type.as_str(),
        input.entity_id.as_str(),
        input.operation.as_str(),
        payload_hash,
    ] {
        hash_part(&mut hasher, part);
    }
    format!("search_outbox:{}", hex::encode(hasher.finalize()))
}

fn hash_part(hasher: &mut Sha256, part: &str) {
    hasher.update(part.len().to_string().as_bytes());
    hasher.update(b":");
    hasher.update(part.as_bytes());
    hasher.update(b";");
}
