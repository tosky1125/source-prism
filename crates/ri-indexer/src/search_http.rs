use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OpenSearchError {
    #[error("OpenSearch request failed: {status} {body}")]
    HttpStatus {
        status: reqwest::StatusCode,
        body: String,
    },
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

#[derive(Debug, Clone)]
pub struct OpenSearchClient {
    base_url: String,
    http: reqwest::Client,
}

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
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            http: reqwest::Client::new(),
        }
    }

    pub async fn health(&self) -> Result<(), OpenSearchError> {
        let response = self
            .http
            .get(format!("{}/_cluster/health", self.base_url))
            .send()
            .await?;
        ok_or_status(response).await
    }

    pub async fn delete_index_if_exists(&self, index: &str) -> Result<(), OpenSearchError> {
        let response = self
            .http
            .delete(format!("{}/{}", self.base_url, index))
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        ok_or_status(response).await
    }

    pub async fn create_index(&self, index: &str) -> Result<(), OpenSearchError> {
        let response = self
            .http
            .put(format!("{}/{}", self.base_url, index))
            .json(&json!({ "settings": { "index": { "number_of_shards": 1 } } }))
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            let body = response.text().await?;
            if body.contains("resource_already_exists_exception") {
                return Ok(());
            }
            return Err(OpenSearchError::HttpStatus {
                status: reqwest::StatusCode::BAD_REQUEST,
                body,
            });
        }
        ok_or_status(response).await
    }

    pub async fn upsert_document(
        &self,
        index: &str,
        document_id: &str,
        payload: &Value,
    ) -> Result<(), OpenSearchError> {
        let response = self
            .http
            .put(format!(
                "{}/{}/_doc/{}?refresh=true",
                self.base_url, index, document_id
            ))
            .json(payload)
            .send()
            .await?;
        ok_or_status(response).await
    }

    pub async fn delete_document(
        &self,
        index: &str,
        document_id: &str,
    ) -> Result<(), OpenSearchError> {
        let response = self
            .http
            .delete(format!(
                "{}/{}/_doc/{}?refresh=true",
                self.base_url, index, document_id
            ))
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        ok_or_status(response).await
    }

    pub async fn count_documents(&self, index: &str) -> Result<i64, OpenSearchError> {
        let response = self
            .http
            .get(format!("{}/{}/_count", self.base_url, index))
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(0);
        }
        if !response.status().is_success() {
            return Err(status_error(response).await);
        }
        let body = response.json::<Value>().await?;
        Ok(body.get("count").and_then(Value::as_i64).unwrap_or(0))
    }

    pub async fn count_documents_for_repo_generation(
        &self,
        index: &str,
        repo_id: &str,
        generation_id: &str,
    ) -> Result<i64, OpenSearchError> {
        let response = self
            .http
            .post(format!("{}/{}/_count", self.base_url, index))
            .json(&json!({
                "query": {
                    "bool": {
                        "filter": [
                            { "term": { "repo_id.keyword": repo_id } },
                            { "term": { "generation_id.keyword": generation_id } }
                        ]
                    }
                }
            }))
            .send()
            .await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(0);
        }
        if !response.status().is_success() {
            return Err(status_error(response).await);
        }
        let body = response.json::<Value>().await?;
        Ok(body.get("count").and_then(Value::as_i64).unwrap_or(0))
    }

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
            return Err(status_error(response).await);
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

async fn ok_or_status(response: reqwest::Response) -> Result<(), OpenSearchError> {
    if response.status().is_success() {
        Ok(())
    } else {
        Err(status_error(response).await)
    }
}

async fn status_error(response: reqwest::Response) -> OpenSearchError {
    let status = response.status();
    let body = response.text().await.unwrap_or_else(|_| String::new());
    OpenSearchError::HttpStatus { status, body }
}
