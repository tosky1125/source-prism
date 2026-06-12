use ri_core::GenerationId;
use ri_symbols::SymbolRecord;
use serde_json::{Value, json};
use sqlx::Row as _;

use crate::{PgSearchSyncStore, SearchSyncError, SearchSyncInput};

const SYMBOL_CHUNK_ENTITY_TYPE: &str = "symbol_chunk";

impl PgSearchSyncStore {
    pub async fn enqueue_symbol_chunks(
        &self,
        repo_id: &str,
        generation_id: &GenerationId,
        symbols: &[SymbolRecord],
        target_index: &str,
    ) -> Result<u64, SearchSyncError> {
        let mut enqueued = 0_u64;
        let generation_id = generation_id.to_string();
        for symbol in symbols {
            let entity_id = symbol_chunk_id(symbol);
            let input = SearchSyncInput::upsert_for_generation(
                repo_id,
                &generation_id,
                SYMBOL_CHUNK_ENTITY_TYPE,
                &entity_id,
                target_index,
                symbol_chunk_payload(repo_id, &generation_id, &entity_id, symbol),
            );
            self.enqueue(&input).await?;
            enqueued = enqueued.saturating_add(1);
        }
        Ok(enqueued)
    }

    pub async fn active_symbol_chunk_count_for_repo(
        &self,
        repo_id: &str,
    ) -> Result<i64, SearchSyncError> {
        let row = sqlx::query(
            r"
            SELECT count(*)::bigint AS count
            FROM search_sync_outbox
            WHERE repo_id = $1
              AND entity_type = $2
              AND operation = 'upsert'
              AND state <> 'cancelled'
              AND generation_id = (
                  SELECT generation_id
                  FROM index_generations
                  WHERE repo_id = $1 AND status = 'succeeded'
                  ORDER BY started_at DESC
                  LIMIT 1
              )
            ",
        )
        .bind(repo_id)
        .bind(SYMBOL_CHUNK_ENTITY_TYPE)
        .fetch_one(&self.pool)
        .await?;
        row.try_get("count").map_err(Into::into)
    }
}

fn symbol_chunk_id(symbol: &SymbolRecord) -> String {
    let kind = json!(symbol.kind);
    format!(
        "chunk:symbol:{}:{}:{}:{}:{}:{}",
        symbol.versioned_symbol_id,
        json_text(&kind),
        symbol.range.start_line,
        symbol.range.start_column,
        symbol.range.end_line,
        symbol.range.end_column
    )
}

fn symbol_chunk_payload(
    repo_id: &str,
    generation_id: &str,
    chunk_id: &str,
    symbol: &SymbolRecord,
) -> Value {
    let language = json!(symbol.language);
    let kind = json!(symbol.kind);
    json!({
        "chunk_id": chunk_id,
        "repo_id": repo_id,
        "generation_id": generation_id,
        "text": symbol_chunk_text(symbol, &language, &kind),
        "symbol": {
            "stable_symbol_id": symbol.stable_symbol_id,
            "versioned_symbol_id": symbol.versioned_symbol_id,
            "file_path": symbol.file_path,
            "language": language,
            "kind": kind,
            "name": symbol.name,
            "fqn": symbol.fqn,
            "range": symbol.range,
        },
    })
}

fn symbol_chunk_text(symbol: &SymbolRecord, language: &Value, kind: &Value) -> String {
    format!(
        "{} {} {} {}",
        symbol.fqn,
        json_text(kind),
        json_text(language),
        symbol.file_path
    )
}

fn json_text(value: &Value) -> &str {
    value.as_str().unwrap_or("unknown")
}

#[cfg(test)]
mod tests {
    use ri_core::{CommitSha, FilePath, Language, RepoId, SymbolKind};
    use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};

    use super::symbol_chunk_id;

    #[test]
    fn symbol_chunk_ids_include_range_to_avoid_document_overwrites()
    -> Result<(), Box<dyn std::error::Error>> {
        let first = symbol_with_range(SymbolRange::new(1, 0, 3, 1))?;
        let second = symbol_with_range(SymbolRange::new(5, 0, 7, 1))?;

        assert_eq!(first.versioned_symbol_id, second.versioned_symbol_id);
        assert_ne!(symbol_chunk_id(&first), symbol_chunk_id(&second));
        Ok(())
    }

    fn symbol_with_range(range: SymbolRange) -> Result<SymbolRecord, ri_core::CoreError> {
        Ok(SymbolRecord::new(
            &RepoId::new("repo")?,
            &CommitSha::new("commit")?,
            FilePath::new("src/invoice.rs")?,
            "hash",
            SymbolSpec::new(
                Language::Rust,
                SymbolKind::Function,
                "apply_tax",
                "InvoiceService::apply_tax",
                range,
            ),
        ))
    }
}
