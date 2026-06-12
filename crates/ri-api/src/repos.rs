use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use ri_core::RepoId;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row as _};

use crate::{AppError, state::AppState};

mod local;

use local::{local_latest_run, local_repo};

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
pub(crate) struct GetRepoResponse {
    status: &'static str,
    kind: &'static str,
    repo: RepoSummary,
    latest_run: Option<RepoLatestRun>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RepoSummary {
    repo_id: String,
    name: String,
    origin_url: Option<String>,
    default_branch: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RepoLatestRun {
    run_id: String,
    commit_sha: String,
    index_kind: String,
    status: String,
    started_at: String,
    finished_at: Option<String>,
    evidence: RepoEvidenceSummary,
}

#[derive(Debug, Serialize)]
pub(crate) struct RepoEvidenceSummary {
    file_manifests: i64,
    symbols: i64,
    graph_nodes: i64,
    graph_edges: i64,
    search_chunks: i64,
    test_cases: i64,
    architecture_entities: i64,
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

pub(crate) async fn get(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<GetRepoResponse>, AppError> {
    let (repo, latest_run) = if let Some(pool) = state.database.pool.as_ref() {
        (
            find_repo(pool, &repo_id).await?,
            latest_run(pool, &repo_id).await?,
        )
    } else {
        (
            local_repo(&repo_id),
            Some(local_latest_run(&state, &repo_id)?),
        )
    };
    Ok(Json(GetRepoResponse {
        status: "ok",
        kind: "repo",
        repo,
        latest_run,
    }))
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

async fn find_repo(pool: &PgPool, repo_id: &str) -> Result<RepoSummary, AppError> {
    let row = sqlx::query(
        r"
        SELECT repo_id, name, origin_url, default_branch
        FROM repos
        WHERE repo_id = $1
        ",
    )
    .bind(repo_id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::RepoNotFound {
        repo_id: repo_id.to_owned(),
    })?;
    Ok(RepoSummary {
        repo_id: row.try_get("repo_id")?,
        name: row.try_get("name")?,
        origin_url: row.try_get("origin_url")?,
        default_branch: row.try_get("default_branch")?,
    })
}

async fn latest_run(pool: &PgPool, repo_id: &str) -> Result<Option<RepoLatestRun>, sqlx::Error> {
    let Some(row) = sqlx::query(
        r"
        SELECT
            g.generation_id,
            g.commit_sha,
            g.index_kind,
            g.status,
            g.started_at::text AS started_at,
            g.finished_at::text AS finished_at,
            (
                SELECT count(*)::bigint FROM file_manifests AS item
                WHERE item.generation_id = g.generation_id
            ) AS file_manifest_count,
            (
                SELECT count(*)::bigint FROM symbols AS item
                WHERE item.generation_id = g.generation_id
            ) AS symbol_count,
            (
                SELECT count(*)::bigint FROM graph_nodes AS item
                WHERE item.generation_id = g.generation_id
            ) AS graph_node_count,
            (
                SELECT count(*)::bigint FROM graph_edges AS item
                WHERE item.generation_id = g.generation_id
            ) AS graph_edge_count,
            (
                SELECT count(*)::bigint FROM search_sync_outbox AS item
                WHERE item.generation_id = g.generation_id
            ) AS search_chunk_count,
            (
                SELECT count(*)::bigint FROM test_cases AS item
                WHERE item.generation_id = g.generation_id
            ) AS test_case_count,
            (
                SELECT count(*)::bigint FROM architecture_entities AS item
                WHERE item.generation_id = g.generation_id
            ) AS architecture_entity_count
        FROM index_generations AS g
        WHERE g.repo_id = $1
        ORDER BY g.started_at DESC
        LIMIT 1
        ",
    )
    .bind(repo_id)
    .fetch_optional(pool)
    .await?
    else {
        return Ok(None);
    };
    Ok(Some(RepoLatestRun {
        run_id: row.try_get("generation_id")?,
        commit_sha: row.try_get("commit_sha")?,
        index_kind: row.try_get("index_kind")?,
        status: row.try_get("status")?,
        started_at: row.try_get("started_at")?,
        finished_at: row.try_get("finished_at")?,
        evidence: RepoEvidenceSummary {
            file_manifests: row.try_get("file_manifest_count")?,
            symbols: row.try_get("symbol_count")?,
            graph_nodes: row.try_get("graph_node_count")?,
            graph_edges: row.try_get("graph_edge_count")?,
            search_chunks: row.try_get("search_chunk_count")?,
            test_cases: row.try_get("test_case_count")?,
            architecture_entities: row.try_get("architecture_entity_count")?,
        },
    }))
}
