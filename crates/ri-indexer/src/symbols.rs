mod codec;
mod store;

pub use store::PgSymbolStore;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum SymbolStoreError {
    #[error("index generation {generation_id} was not found")]
    GenerationNotFound { generation_id: String },
    #[error("invalid stored {field}: {value}")]
    InvalidStoredEnum { field: &'static str, value: String },
    #[error("invalid range value: {field}={value}")]
    InvalidRangeValue { field: &'static str, value: u32 },
    #[error("invalid stored range value: {field}={value}")]
    InvalidStoredRange { field: &'static str, value: i32 },
    #[error(transparent)]
    Core(#[from] ri_core::CoreError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}
