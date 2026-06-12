use axum::{
    Json,
    extract::{Path, State},
};
use ri_worker::{DeadLetterJob, JobQueue, PgJobStore};
use serde::Serialize;
use sqlx::PgPool;

use crate::{AppError, state::AppState};

const DEAD_LETTER_LIMIT: i64 = 50;

#[derive(Debug, Serialize)]
pub(crate) struct RepoDeadLettersResponse {
    status: &'static str,
    kind: &'static str,
    repo_id: String,
    dead_letter_count: usize,
    dead_letters: Vec<DeadLetterJob>,
}

pub(crate) async fn list(
    State(state): State<AppState>,
    Path(repo_id): Path<String>,
) -> Result<Json<RepoDeadLettersResponse>, AppError> {
    let Some(pool) = state.database.pool.as_ref() else {
        return Ok(Json(response(repo_id, Vec::new())));
    };
    ensure_repo_exists(pool, &repo_id).await?;
    let dead_letters = PgJobStore::new(pool.clone(), JobQueue::default())
        .dead_letters_for_repo(&repo_id, DEAD_LETTER_LIMIT)
        .await?;
    Ok(Json(response(repo_id, dead_letters)))
}

fn response(repo_id: String, dead_letters: Vec<DeadLetterJob>) -> RepoDeadLettersResponse {
    RepoDeadLettersResponse {
        status: "ok",
        kind: "repo_dead_letters",
        repo_id,
        dead_letter_count: dead_letters.len(),
        dead_letters,
    }
}

async fn ensure_repo_exists(pool: &PgPool, repo_id: &str) -> Result<(), AppError> {
    let exists = sqlx::query_scalar::<_, bool>(
        r"
        SELECT EXISTS(
            SELECT 1 FROM repos WHERE repo_id = $1
        )
        ",
    )
    .bind(repo_id)
    .fetch_one(pool)
    .await?;
    if exists {
        Ok(())
    } else {
        Err(AppError::RepoNotFound {
            repo_id: repo_id.to_owned(),
        })
    }
}
