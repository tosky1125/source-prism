use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{OpenSearchClient, OpenSearchError};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct OpenSearchTextHit {
    pub chunk_id: String,
    pub repo_id: String,
    pub text: String,
    pub symbol: Value,
    pub score: f64,
}

impl OpenSearchClient {
    pub async fn search_text(
        &self,
        index: &str,
        repo_id: &str,
        query: &str,
        limit: usize,
    ) -> Result<Vec<OpenSearchTextHit>, OpenSearchError> {
        let response = self
            .http
            .post(format!("{}/{}/_search", self.base_url, index))
            .json(&json!({
                "size": limit,
                "query": {
                    "bool": {
                        "must": [{ "match": { "text": query } }],
                        "filter": [{ "term": { "repo_id.keyword": repo_id } }]
                    }
                }
            }))
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(Vec::new());
        }
        if !response.status().is_success() {
            return Err(search_status_error(response).await);
        }
        let body = response.json::<SearchResponse>().await?;
        Ok(body
            .hits
            .hits
            .into_iter()
            .filter_map(SearchHitEnvelope::into_text_hit)
            .collect())
    }
}

async fn search_status_error(response: reqwest::Response) -> OpenSearchError {
    let status = response.status();
    let body = response.text().await.unwrap_or_else(|_| String::new());
    OpenSearchError::HttpStatus { status, body }
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    hits: SearchHitList,
}

#[derive(Debug, Deserialize)]
struct SearchHitList {
    hits: Vec<SearchHitEnvelope>,
}

#[derive(Debug, Deserialize)]
struct SearchHitEnvelope {
    #[serde(rename = "_score")]
    score: Option<f64>,
    #[serde(rename = "_source")]
    source: Option<SearchHitSource>,
}

#[derive(Debug, Deserialize)]
struct SearchHitSource {
    chunk_id: String,
    repo_id: String,
    text: String,
    symbol: Value,
}

impl SearchHitEnvelope {
    fn into_text_hit(self) -> Option<OpenSearchTextHit> {
        let source = self.source?;
        Some(OpenSearchTextHit {
            chunk_id: source.chunk_id,
            repo_id: source.repo_id,
            text: source.text,
            symbol: source.symbol,
            score: self.score.unwrap_or(0.0),
        })
    }
}
