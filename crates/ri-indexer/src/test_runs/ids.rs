use ri_behavior::TestCaseResult;
use ri_core::GenerationId;
use sha2::{Digest, Sha256};

use super::store::StoredGeneration;

pub(super) fn test_run_id(
    generation: &StoredGeneration,
    generation_id: &GenerationId,
    source_path: &str,
) -> String {
    digest(
        "trun",
        &[
            generation.repo_id.as_str(),
            generation.commit_sha.as_str(),
            generation_id.as_str(),
            source_path,
        ],
    )
}

pub(super) fn test_result_id(test_run_id: &str, result: &TestCaseResult) -> String {
    digest(
        "tres",
        &[test_run_id, result.suite_name.as_str(), result.fqn.as_str()],
    )
}

fn digest(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part.len().to_string().as_bytes());
        hasher.update(b":");
        hasher.update(part.as_bytes());
        hasher.update(b";");
    }
    format!("{prefix}:{}", hex::encode(hasher.finalize()))
}
