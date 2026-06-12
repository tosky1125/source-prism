use async_trait::async_trait;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::model::{
    EnqueueJob, JobId, JobKind, JobLease, JobQueue, JobRecord, JobState, LeasedJob, WorkerId,
};
use crate::runtime::{JobError, JobStore};

#[derive(Debug, Clone, Default)]
pub struct MemoryJobStore {
    inner: Arc<Mutex<MemoryState>>,
}

impl MemoryJobStore {
    pub fn get(&self, job_id: JobId) -> Result<JobRecord, JobError> {
        let state = self.inner.lock().map_err(|_| JobError::StoreLockPoisoned)?;
        let Some(job) = state.jobs.get(&job_id) else {
            return Err(JobError::NoJobAvailable);
        };
        let record = job.record();
        drop(state);
        Ok(record)
    }

    pub fn len(&self) -> usize {
        self.inner.lock().map_or(0, |state| state.jobs.len())
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn advance_by(&self, duration: Duration) -> Result<(), JobError> {
        let mut state = self.inner.lock().map_err(|_| JobError::StoreLockPoisoned)?;
        state.now = state
            .now
            .checked_add(duration)
            .ok_or(JobError::ClockOverflow)?;
        drop(state);
        Ok(())
    }
}

#[derive(Debug)]
struct MemoryState {
    now: Instant,
    jobs: BTreeMap<JobId, MemoryJob>,
}

impl Default for MemoryState {
    fn default() -> Self {
        Self {
            now: Instant::now(),
            jobs: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
struct MemoryJob {
    job_id: JobId,
    queue: JobQueue,
    kind: JobKind,
    payload: serde_json::Value,
    state: JobState,
    idempotency_key: Option<String>,
    priority: i32,
    run_after: Instant,
    attempt_count: i32,
    max_attempts: i32,
    backoff: Duration,
    leased_by: Option<WorkerId>,
    leased_until: Option<Instant>,
}

impl MemoryJob {
    const fn record(&self) -> JobRecord {
        JobRecord {
            job_id: self.job_id,
            state: self.state,
            attempt_count: self.attempt_count,
        }
    }
}

#[async_trait]
impl JobStore for MemoryJobStore {
    async fn enqueue(&self, request: EnqueueJob) -> Result<JobRecord, JobError> {
        let record = {
            let mut state = self.inner.lock().map_err(|_| JobError::StoreLockPoisoned)?;
            if let Some(existing) = find_idempotent(&state, &request) {
                return Ok(existing.record());
            }
            let job = MemoryJob {
                job_id: JobId::new(),
                queue: request.queue,
                kind: request.kind,
                payload: request.payload,
                state: JobState::Queued,
                idempotency_key: request.idempotency_key,
                priority: request.priority,
                run_after: state.now,
                attempt_count: 0,
                max_attempts: request.max_attempts,
                backoff: request.backoff.delay(),
                leased_by: None,
                leased_until: None,
            };
            let record = job.record();
            state.jobs.insert(job.job_id, job);
            record
        };
        Ok(record)
    }

    async fn lease_next(
        &self,
        worker_id: &WorkerId,
        lease_timeout: Duration,
    ) -> Result<Option<LeasedJob>, JobError> {
        let mut state = self.inner.lock().map_err(|_| JobError::StoreLockPoisoned)?;
        let Some(job_id) = next_ready_job(&state) else {
            return Ok(None);
        };
        let now = state.now;
        let Some(job) = state.jobs.get_mut(&job_id) else {
            return Ok(None);
        };
        job.state = JobState::Leased;
        job.attempt_count = job
            .attempt_count
            .checked_add(1)
            .ok_or(JobError::AttemptOverflow)?;
        job.leased_by = Some(worker_id.clone());
        job.leased_until = Some(
            now.checked_add(lease_timeout)
                .ok_or(JobError::ClockOverflow)?,
        );
        let leased = LeasedJob {
            lease: JobLease {
                job_id,
                attempt_no: job.attempt_count,
            },
            kind: job.kind.clone(),
            payload: job.payload.clone(),
        };
        drop(state);
        Ok(Some(leased))
    }

    async fn succeed(&self, lease: JobLease) -> Result<(), JobError> {
        {
            let mut state = self.inner.lock().map_err(|_| JobError::StoreLockPoisoned)?;
            if let Some(job) = state.jobs.get_mut(&lease.job_id) {
                job.state = JobState::Succeeded;
                job.leased_by = None;
                job.leased_until = None;
            }
        }
        Ok(())
    }

    async fn fail(&self, lease: JobLease, _error: &str) -> Result<(), JobError> {
        {
            let mut state = self.inner.lock().map_err(|_| JobError::StoreLockPoisoned)?;
            let now = state.now;
            if let Some(job) = state.jobs.get_mut(&lease.job_id) {
                job.leased_by = None;
                job.leased_until = None;
                if job.attempt_count >= job.max_attempts {
                    job.state = JobState::DeadLettered;
                } else {
                    job.state = JobState::Failed;
                    job.run_after = now
                        .checked_add(job.backoff)
                        .ok_or(JobError::ClockOverflow)?;
                }
            }
        }
        Ok(())
    }

    async fn cancel(&self, job_id: JobId) -> Result<(), JobError> {
        {
            let mut state = self.inner.lock().map_err(|_| JobError::StoreLockPoisoned)?;
            if let Some(job) = state.jobs.get_mut(&job_id) {
                job.state = JobState::Cancelled;
                job.leased_by = None;
                job.leased_until = None;
            }
        }
        Ok(())
    }
}

fn find_idempotent<'a>(state: &'a MemoryState, request: &EnqueueJob) -> Option<&'a MemoryJob> {
    let key = request.idempotency_key.as_ref()?;
    state.jobs.values().find(|job| {
        job.queue == request.queue
            && job.kind == request.kind
            && job.idempotency_key.as_ref() == Some(key)
    })
}

fn next_ready_job(state: &MemoryState) -> Option<JobId> {
    state
        .jobs
        .values()
        .filter(|job| is_ready(job, state.now))
        .max_by_key(|job| (job.priority, std::cmp::Reverse(job.run_after), job.job_id))
        .map(|job| job.job_id)
}

fn is_ready(job: &MemoryJob, now: Instant) -> bool {
    match job.state {
        JobState::Queued | JobState::Failed => job.run_after <= now,
        JobState::Leased => job
            .leased_until
            .is_some_and(|leased_until| leased_until <= now),
        JobState::Succeeded | JobState::DeadLettered | JobState::Cancelled => false,
    }
}
