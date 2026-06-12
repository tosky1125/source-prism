#![allow(
    missing_docs,
    reason = "Integration tests use scenario names instead of API docs."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

use ri_behavior::parse_junit_xml;
use ri_indexer::{PgGenerationStore, PgTestRunStore};
use sqlx::PgPool;
use uuid::Uuid;

#[tokio::test]
async fn active_test_runs_returns_junit_results_for_repo() -> Result<(), Box<dyn std::error::Error>>
{
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "test_results",
            Some("test"),
        )
        .await?;
    let report = parse_junit_xml(
        r#"<testsuite name="invoice"><testcase classname="InvoiceTest" name="adds_rate"/></testsuite>"#,
    )?;
    let store = PgTestRunStore::new(pool.clone());
    let outcome = store
        .replace_junit_run_for_generation(&generation.generation_id, "junit.xml", &report)
        .await?;
    PgGenerationStore::new(pool.clone())
        .finish_generation(&generation.generation_id)
        .await?;

    let runs = store.active_test_runs_for_repo(&fixture.repo_id).await?;

    assert_eq!(outcome.result_count, 1);
    assert_eq!(runs.len(), 1);
    let run = runs
        .first()
        .ok_or_else(|| std::io::Error::other("missing test run"))?;
    assert_eq!(run.total_count, 1);
    assert_eq!(run.passed_count, 1);
    let result = run
        .results
        .first()
        .ok_or_else(|| std::io::Error::other("missing test result"))?;
    assert_eq!(result.fqn, "InvoiceTest::adds_rate");
    fixture.cleanup(&pool).await?;
    Ok(())
}

#[tokio::test]
async fn active_test_runs_preserves_pytest_framework_for_repo()
-> Result<(), Box<dyn std::error::Error>> {
    let Some(pool) = test_pool().await? else {
        return Ok(());
    };
    let fixture = Fixture::create(&pool).await?;
    let generation = PgGenerationStore::new(pool.clone())
        .begin_generation(
            &fixture.repo_id,
            &fixture.commit_sha,
            "test_results",
            Some("test"),
        )
        .await?;
    let report = parse_junit_xml(
        r#"<testsuite name="pytest"><testcase classname="test_invoice" name="test_adds_rate"/></testsuite>"#,
    )?;
    PgTestRunStore::new(pool.clone())
        .replace_pytest_run_for_generation(&generation.generation_id, "pytest.json", &report)
        .await?;
    PgGenerationStore::new(pool.clone())
        .finish_generation(&generation.generation_id)
        .await?;

    let runs = PgTestRunStore::new(pool.clone())
        .active_test_runs_for_repo(&fixture.repo_id)
        .await?;

    let run = runs
        .first()
        .ok_or_else(|| std::io::Error::other("missing test run"))?;
    assert_eq!(run.framework, "pytest");
    assert_eq!(run.results.len(), 1);
    fixture.cleanup(&pool).await?;
    Ok(())
}

#[derive(Debug)]
struct Fixture {
    repo_id: String,
    commit_sha: String,
}

impl Fixture {
    async fn create(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let suffix = Uuid::now_v7();
        let fixture = Self {
            repo_id: format!("repo-{suffix}"),
            commit_sha: format!("commit-{suffix}"),
        };
        sqlx::query("INSERT INTO repos (repo_id, name) VALUES ($1, $1)")
            .bind(&fixture.repo_id)
            .execute(pool)
            .await?;
        sqlx::query("INSERT INTO commits (repo_id, commit_sha) VALUES ($1, $2)")
            .bind(&fixture.repo_id)
            .bind(&fixture.commit_sha)
            .execute(pool)
            .await?;
        Ok(fixture)
    }

    async fn cleanup(&self, pool: &PgPool) -> Result<(), sqlx::Error> {
        for table in [
            "test_results",
            "test_runs",
            "index_generations",
            "commits",
            "repos",
        ] {
            sqlx::query(&format!("DELETE FROM {table} WHERE repo_id = $1"))
                .bind(&self.repo_id)
                .execute(pool)
                .await?;
        }
        Ok(())
    }
}

async fn test_pool() -> Result<Option<PgPool>, sqlx::Error> {
    let Ok(database_url) = std::env::var("DATABASE_URL") else {
        return Ok(None);
    };
    PgPool::connect(database_url.as_str()).await.map(Some)
}
