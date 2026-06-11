use ri_symbols::SymbolRecord;
use sha2::{Digest, Sha256};

const NODE_TYPE_FILE: &str = "file";
const NODE_TYPE_SYMBOL: &str = "symbol";

pub fn file_node_id(repo_id: &str, commit_sha: &str, file_path: &str) -> String {
    node_id(repo_id, commit_sha, NODE_TYPE_FILE, file_path)
}

pub fn symbol_node_id(symbol: &SymbolRecord) -> String {
    node_id(
        "symbol",
        "versioned",
        NODE_TYPE_SYMBOL,
        symbol.versioned_symbol_id.as_str(),
    )
}

pub fn contains_edge_id(
    repo_id: &str,
    commit_sha: &str,
    source_node_id: &str,
    target_node_id: &str,
    edge_type: &str,
) -> String {
    prefixed_digest(
        "ge",
        &[
            repo_id,
            commit_sha,
            source_node_id,
            target_node_id,
            edge_type,
        ],
    )
}

fn node_id(repo_id: &str, commit_sha: &str, node_type: &str, subject_id: &str) -> String {
    prefixed_digest("gn", &[repo_id, commit_sha, node_type, subject_id])
}

fn prefixed_digest(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.len().to_string().as_bytes());
        hasher.update(b":");
        hasher.update(part.as_bytes());
        hasher.update(b";");
    }
    format!("{prefix}:{}", hex::encode(hasher.finalize()))
}
