use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum EmbeddingCacheError {
    #[error("empty embedding cache field: {field}")]
    EmptyField { field: &'static str },
    #[error("embedding dimensions must be positive")]
    InvalidDimensions,
    #[error("embedding vector dimension mismatch: expected={expected} actual={actual}")]
    DimensionMismatch { expected: i32, actual: usize },
    #[error("embedding vector contains non-finite value")]
    NonFiniteVector,
    #[error("invalid embedding vector component: {value}")]
    InvalidVectorComponent { value: String },
    #[error("invalid embedding byte length: {bytes}")]
    InvalidEmbeddingBytes { bytes: usize },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbeddingCacheInput {
    pub provider: String,
    pub model: String,
    pub input_kind: String,
    pub input: String,
    pub dimensions: i32,
}

impl EmbeddingCacheInput {
    pub fn parse(
        provider: &str,
        model: &str,
        input_kind: &str,
        input: &str,
        dimensions: i32,
    ) -> Result<Self, EmbeddingCacheError> {
        require_non_empty(provider, "provider")?;
        require_non_empty(model, "model")?;
        require_non_empty(input_kind, "input_kind")?;
        require_non_empty(input, "input")?;
        if dimensions <= 0 {
            return Err(EmbeddingCacheError::InvalidDimensions);
        }
        Ok(Self {
            provider: provider.to_owned(),
            model: model.to_owned(),
            input_kind: input_kind.to_owned(),
            input: input.to_owned(),
            dimensions,
        })
    }

    pub fn input_sha256(&self) -> String {
        hex_sha256(self.input.as_bytes())
    }

    pub fn cache_key(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.provider.as_bytes());
        hasher.update([0]);
        hasher.update(self.model.as_bytes());
        hasher.update([0]);
        hasher.update(self.input_sha256().as_bytes());
        hasher.update([0]);
        hasher.update(self.dimensions.to_string().as_bytes());
        format!("emb:{}", hex::encode(hasher.finalize()))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbeddingVector {
    pub values: Vec<f32>,
}

impl EmbeddingVector {
    pub fn from_f32(values: Vec<f32>) -> Result<Self, EmbeddingCacheError> {
        if values.is_empty() {
            return Err(EmbeddingCacheError::InvalidDimensions);
        }
        if values.iter().any(|value| !value.is_finite()) {
            return Err(EmbeddingCacheError::NonFiniteVector);
        }
        Ok(Self { values })
    }

    pub fn parse_csv(raw: &str) -> Result<Self, EmbeddingCacheError> {
        let mut values = Vec::new();
        for component in raw.split(',').map(str::trim) {
            require_non_empty(component, "vector")?;
            let value = component.parse::<f32>().map_err(|_| {
                EmbeddingCacheError::InvalidVectorComponent {
                    value: component.to_owned(),
                }
            })?;
            values.push(value);
        }
        Self::from_f32(values)
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub(crate) fn encode_le(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.values.len().saturating_mul(4));
        for value in &self.values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    pub(crate) fn decode_le(bytes: &[u8]) -> Result<Self, EmbeddingCacheError> {
        let chunks = bytes.chunks_exact(4);
        if !chunks.remainder().is_empty() {
            return Err(EmbeddingCacheError::InvalidEmbeddingBytes { bytes: bytes.len() });
        }
        let mut values = Vec::with_capacity(bytes.len() / 4);
        for chunk in chunks {
            let array = <[u8; 4]>::try_from(chunk)
                .map_err(|_| EmbeddingCacheError::InvalidEmbeddingBytes { bytes: bytes.len() })?;
            values.push(f32::from_le_bytes(array));
        }
        Self::from_f32(values)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbeddingCacheEntry {
    pub cache_key: String,
    pub provider: String,
    pub model: String,
    pub input_sha256: String,
    pub input_kind: String,
    pub dimensions: i32,
    pub vector: EmbeddingVector,
    pub metadata: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EmbeddingCacheWrite {
    pub cache_hit: bool,
    pub entry: EmbeddingCacheEntry,
}

fn require_non_empty(value: &str, field: &'static str) -> Result<(), EmbeddingCacheError> {
    if value.trim().is_empty() {
        Err(EmbeddingCacheError::EmptyField { field })
    } else {
        Ok(())
    }
}

fn hex_sha256(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
