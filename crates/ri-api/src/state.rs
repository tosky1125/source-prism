use ri_context::extract_repo_symbols;
use ri_git::LocalManifest;
use ri_indexer::PgSymbolStore;
use ri_symbols::SymbolRecord;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::{borrow::Cow, env, path::PathBuf, sync::Arc, time::Duration};

use crate::{ApiError, AppError, RepoFile};

#[derive(Debug, Clone)]
pub struct AppState {
    pub(crate) database: DatabaseState,
    pub(crate) opensearch_url: Option<String>,
    pub(crate) http_client: reqwest::Client,
    context_repo_path: PathBuf,
    repo_files: Option<Arc<[RepoFile]>>,
    context_symbols: Option<Arc<[SymbolRecord]>>,
}

impl AppState {
    pub fn from_env() -> Result<Self, ApiError> {
        Ok(Self {
            database: database_pool(),
            opensearch_url: env::var("OPENSEARCH_URL").ok(),
            http_client: http_client()?,
            context_repo_path: env::var("SOURCE_PRISM_REPO")
                .map_or_else(|_| PathBuf::from("."), PathBuf::from),
            repo_files: None,
            context_symbols: None,
        })
    }

    pub fn for_test_symbols(symbols: Vec<SymbolRecord>) -> Result<Self, ApiError> {
        Ok(Self {
            database: DatabaseState {
                configured: false,
                pool: None,
            },
            opensearch_url: None,
            http_client: http_client()?,
            context_repo_path: PathBuf::from("."),
            repo_files: None,
            context_symbols: Some(Arc::from(symbols.into_boxed_slice())),
        })
    }

    pub fn for_test_files(files: Vec<RepoFile>) -> Result<Self, ApiError> {
        Ok(Self {
            database: DatabaseState {
                configured: false,
                pool: None,
            },
            opensearch_url: None,
            http_client: http_client()?,
            context_repo_path: PathBuf::from("."),
            repo_files: Some(Arc::from(files.into_boxed_slice())),
            context_symbols: None,
        })
    }

    pub(crate) fn repo_files(&self) -> Result<Cow<'_, [RepoFile]>, ri_git::GitError> {
        if let Some(files) = self.repo_files.as_ref() {
            return Ok(Cow::Borrowed(files.as_ref()));
        }
        let manifest = LocalManifest::extract(&self.context_repo_path)?;
        let files = manifest
            .files()
            .iter()
            .map(RepoFile::from_manifest)
            .collect::<Vec<_>>();
        Ok(Cow::Owned(files))
    }

    pub(crate) fn context_repo_path(&self) -> &std::path::Path {
        self.context_repo_path.as_path()
    }

    pub(crate) fn context_symbols(
        &self,
    ) -> Result<Cow<'_, [SymbolRecord]>, ri_context::ContextError> {
        if let Some(symbols) = self.context_symbols.as_ref() {
            return Ok(Cow::Borrowed(symbols.as_ref()));
        }
        extract_repo_symbols(&self.context_repo_path).map(Cow::Owned)
    }

    pub(crate) async fn symbols_for_optional_repo(
        &self,
        repo_id: Option<&str>,
    ) -> Result<Vec<SymbolRecord>, AppError> {
        let Some(repo_id) = repo_id else {
            return Ok(self.context_symbols()?.into_owned());
        };
        let repo_id = repo_id.trim();
        if repo_id.is_empty() {
            return Err(AppError::Validation("repo_id must not be empty".to_owned()));
        }
        let Some(pool) = self.database.pool.as_ref() else {
            return Err(AppError::DatabaseNotConfigured);
        };
        PgSymbolStore::new(pool.clone())
            .active_symbols_for_repo(repo_id)
            .await
            .map_err(AppError::from)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DatabaseState {
    pub(crate) configured: bool,
    pub(crate) pool: Option<PgPool>,
}

fn database_pool() -> DatabaseState {
    let Ok(database_url) = env::var("DATABASE_URL") else {
        return DatabaseState {
            configured: false,
            pool: None,
        };
    };
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect_lazy(database_url.as_str())
        .ok();
    DatabaseState {
        configured: true,
        pool,
    }
}

fn http_client() -> Result<reqwest::Client, ApiError> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?)
}
