use async_trait::async_trait;
use std::time::Duration;
use uuid::Error as UuidError;

use crate::model::{
    EnqueueJob, JobId, JobLease, JobRecord, LeaseConfig, LeasedJob, RunOnceOutcome, WorkerId,
};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum JobError {
    #[error("worker job names must not be empty")]
    EmptyName,
    #[error("invalid job id")]
    InvalidJobId { source: UuidError },
    #[error("invalid job state {state}")]
    InvalidState { state: String },
    #[error("invalid max_attempts {max_attempts}; expected a positive value")]
    InvalidMaxAttempts { max_attempts: i32 },
    #[error("duration is too large for SQL interval seconds")]
    DurationTooLarge,
    #[error("job attempt count overflowed")]
    AttemptOverflow,
    #[error("job clock arithmetic overflowed")]
    ClockOverflow,
    #[error("no job is available to lease")]
    NoJobAvailable,
    #[error("in-memory job store lock is poisoned")]
    StoreLockPoisoned,
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}

#[async_trait]
pub trait JobStore: Clone + Send + Sync + 'static {
    async fn enqueue(&self, request: EnqueueJob) -> Result<JobRecord, JobError>;

    async fn lease_next(
        &self,
        worker_id: &WorkerId,
        lease_timeout: Duration,
    ) -> Result<Option<LeasedJob>, JobError>;

    async fn succeed(&self, lease: JobLease) -> Result<(), JobError>;

    async fn fail(&self, lease: JobLease, error: &str) -> Result<(), JobError>;

    async fn cancel(&self, job_id: JobId) -> Result<(), JobError>;
}

#[derive(Debug, Clone)]
pub struct JobRuntime<S> {
    store: S,
    worker_id: WorkerId,
    lease_config: LeaseConfig,
}

impl<S> JobRuntime<S>
where
    S: JobStore,
{
    pub const fn new(store: S, worker_id: WorkerId, lease_config: LeaseConfig) -> Self {
        Self {
            store,
            worker_id,
            lease_config,
        }
    }

    pub async fn enqueue(&self, request: EnqueueJob) -> Result<JobRecord, JobError> {
        validate_enqueue(&request)?;
        self.store.enqueue(request).await
    }

    pub async fn lease_next(&self) -> Result<Option<LeasedJob>, JobError> {
        self.store
            .lease_next(&self.worker_id, self.lease_config.timeout())
            .await
    }

    pub async fn require_lease(&self) -> Result<LeasedJob, JobError> {
        self.lease_next().await?.ok_or(JobError::NoJobAvailable)
    }

    pub async fn succeed(&self, lease: JobLease) -> Result<(), JobError> {
        self.store.succeed(lease).await
    }

    pub async fn fail(&self, lease: JobLease, error: &str) -> Result<(), JobError> {
        self.store.fail(lease, error).await
    }

    pub async fn cancel(&self, job_id: JobId) -> Result<(), JobError> {
        self.store.cancel(job_id).await
    }

    pub async fn run_once(&self) -> Result<RunOnceOutcome, JobError> {
        let Some(job) = self.lease_next().await? else {
            return Ok(RunOnceOutcome {
                processed: false,
                job_id: None,
            });
        };
        if job.kind.is_noop() {
            self.succeed(job.lease).await?;
        } else {
            self.fail(job.lease, "unsupported job kind").await?;
        }
        Ok(RunOnceOutcome {
            processed: true,
            job_id: Some(job.lease.job_id),
        })
    }
}

const fn validate_enqueue(request: &EnqueueJob) -> Result<(), JobError> {
    if request.max_attempts <= 0 {
        return Err(JobError::InvalidMaxAttempts {
            max_attempts: request.max_attempts,
        });
    }
    Ok(())
}
