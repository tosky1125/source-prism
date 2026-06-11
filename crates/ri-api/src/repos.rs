use axum::{Json, extract::State, http::StatusCode};
use ri_core::RepoId;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::{AppError, state::AppState};

#[derive(Debug, Deserialize)]
pub(crate) struct CreateRepoRequest {
    repo_id: Option<String>,
    name: String,
    origin_url: Option<String>,
    default_branch: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct CreateRepoResponse {
    status: &'static str,
    kind: &'static str,
    persisted: bool,
    repo: RepoSummary,
}

#[derive(Debug, Serialize)]
pub(crate) struct RepoSummary {
    repo_id: String,
    name: String,
    origin_url: Option<String>,
    default_branch: Option<String>,
}

pub(crate) async fn create(
    State(state): State<AppState>,
    Json(request): Json<CreateRepoRequest>,
) -> Result<(StatusCode, Json<CreateRepoResponse>), AppError> {
    let repo = parse_repo(&request)?;
    let persisted = if let Some(pool) = state.database.pool.as_ref() {
        upsert_repo(pool, &repo).await?;
        true
    } else {
        false
    };
    Ok((
        StatusCode::CREATED,
        Json(CreateRepoResponse {
            status: "ok",
            kind: "repo",
            persisted,
            repo,
        }),
    ))
}

fn parse_repo(request: &CreateRepoRequest) -> Result<RepoSummary, AppError> {
    let name = non_empty(&request.name, "name")?;
    let default_id = format!("local:{name}");
    let repo_id = request.repo_id.as_deref().unwrap_or(default_id.as_str());
    let repo_id = RepoId::new(repo_id)
        .map_err(|error| AppError::Validation(error.to_string()))?
        .to_string();
    Ok(RepoSummary {
        repo_id,
        name,
        origin_url: optional_non_empty(request.origin_url.as_deref()),
        default_branch: optional_non_empty(request.default_branch.as_deref()),
    })
}

fn non_empty(value: &str, field: &'static str) -> Result<String, AppError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(AppError::Validation(format!("{field} must not be empty")));
    }
    Ok(trimmed.to_owned())
}

fn optional_non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|trimmed| !trimmed.is_empty())
        .map(str::to_owned)
}

async fn upsert_repo(pool: &PgPool, repo: &RepoSummary) -> Result<(), sqlx::Error> {
    sqlx::query(
        r"
        INSERT INTO repos (repo_id, name, origin_url, default_branch)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (repo_id) DO UPDATE
        SET name = EXCLUDED.name,
            origin_url = EXCLUDED.origin_url,
            default_branch = EXCLUDED.default_branch,
            updated_at = now()
        ",
    )
    .bind(&repo.repo_id)
    .bind(&repo.name)
    .bind(&repo.origin_url)
    .bind(&repo.default_branch)
    .execute(pool)
    .await?;
    Ok(())
}
