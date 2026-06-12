#![allow(
    missing_docs,
    reason = "T9 exposes an initial worker runtime contract before external API docs exist."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx TLS dependencies currently pull duplicate platform crates outside this crate's control."
)]

mod memory;
mod model;
mod pg;
mod runtime;

pub use memory::MemoryJobStore;
pub use model::{
    Backoff, EnqueueJob, JobId, JobKind, JobLease, JobQueue, JobRecord, JobState, LeaseConfig,
    LeasedJob, RunOnceOutcome, RunPollsOutcome, WorkerId,
};
pub use pg::PgJobStore;
pub use runtime::{JobError, JobRuntime, JobStore};
