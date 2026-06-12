use serde_json::Value;
use std::fmt::{Display, Formatter};
use std::time::Duration;
use uuid::Uuid;

use crate::runtime::JobError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobId(Uuid);

impl JobId {
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }

    pub fn parse(raw: &str) -> Result<Self, JobError> {
        Uuid::parse_str(raw)
            .map(Self)
            .map_err(|source| JobError::InvalidJobId { source })
    }
}

impl Default for JobId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for JobId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobQueue(String);

impl JobQueue {
    pub fn parse(raw: &str) -> Result<Self, JobError> {
        parse_name(raw).map(Self)
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self("default".to_owned())
    }
}

impl Display for JobQueue {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct JobKind(String);

impl JobKind {
    pub fn parse(raw: &str) -> Result<Self, JobError> {
        parse_name(raw).map(Self)
    }

    pub fn is_noop(&self) -> bool {
        self.0 == "noop"
    }

    pub fn is_search_sync_once(&self) -> bool {
        self.0 == "search.sync_once"
    }
}

impl Display for JobKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WorkerId(String);

impl WorkerId {
    pub fn parse(raw: &str) -> Result<Self, JobError> {
        parse_name(raw).map(Self)
    }
}

impl Display for WorkerId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum JobState {
    Queued,
    Leased,
    Succeeded,
    Failed,
    DeadLettered,
    Cancelled,
}

impl JobState {
    pub fn parse(raw: &str) -> Result<Self, JobError> {
        match raw {
            "queued" => Ok(Self::Queued),
            "leased" => Ok(Self::Leased),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "dead_lettered" => Ok(Self::DeadLettered),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(JobError::InvalidState {
                state: other.to_owned(),
            }),
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Leased => "leased",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::DeadLettered => "dead_lettered",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Backoff {
    delay: Duration,
}

impl Backoff {
    pub const fn fixed(delay: Duration) -> Self {
        Self { delay }
    }

    pub const fn delay(self) -> Duration {
        self.delay
    }
}

impl Default for Backoff {
    fn default() -> Self {
        Self::fixed(Duration::from_secs(30))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeaseConfig {
    timeout: Duration,
}

impl LeaseConfig {
    pub const fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    pub const fn for_tests(timeout: Duration) -> Self {
        Self::new(timeout)
    }

    pub const fn timeout(self) -> Duration {
        self.timeout
    }
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct EnqueueJob {
    pub queue: JobQueue,
    pub kind: JobKind,
    pub payload: Value,
    pub idempotency_key: Option<String>,
    pub priority: i32,
    pub max_attempts: i32,
    pub backoff: Backoff,
}

impl EnqueueJob {
    pub fn new(queue: JobQueue, kind: JobKind, payload: Value) -> Self {
        Self {
            queue,
            kind,
            payload,
            idempotency_key: None,
            priority: 0,
            max_attempts: 3,
            backoff: Backoff::default(),
        }
    }

    #[must_use]
    pub fn with_idempotency_key(mut self, key: &str) -> Self {
        self.idempotency_key = Some(key.to_owned());
        self
    }

    #[must_use]
    pub const fn with_max_attempts(mut self, max_attempts: i32) -> Self {
        self.max_attempts = max_attempts;
        self
    }

    #[must_use]
    pub const fn with_backoff(mut self, backoff: Backoff) -> Self {
        self.backoff = backoff;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct JobRecord {
    pub job_id: JobId,
    pub state: JobState,
    pub attempt_count: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct JobLease {
    pub job_id: JobId,
    pub attempt_no: i32,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct LeasedJob {
    pub lease: JobLease,
    pub kind: JobKind,
    pub payload: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RunOnceOutcome {
    pub processed: bool,
    pub job_id: Option<JobId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct RunPollsOutcome {
    pub polls: u64,
    pub processed: u64,
}

fn parse_name(raw: &str) -> Result<String, JobError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(JobError::EmptyName);
    }
    Ok(trimmed.to_owned())
}
