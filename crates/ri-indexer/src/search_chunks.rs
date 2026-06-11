use ri_core::GenerationId;
use ri_symbols::SymbolRecord;
use serde_json::{Value, json};

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
                symbol_chunk_payload(repo_id, &entity_id, symbol),
            );
            self.enqueue(&input).await?;
            enqueued = enqueued.saturating_add(1);
        }
        Ok(enqueued)
    }
}

fn symbol_chunk_id(symbol: &SymbolRecord) -> String {
    format!("chunk:symbol:{}", symbol.versioned_symbol_id)
}

fn symbol_chunk_payload(repo_id: &str, chunk_id: &str, symbol: &SymbolRecord) -> Value {
    let language = json!(symbol.language);
    let kind = json!(symbol.kind);
    json!({
        "chunk_id": chunk_id,
        "repo_id": repo_id,
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
