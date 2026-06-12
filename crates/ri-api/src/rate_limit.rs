use axum::{
    Json,
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::ApiError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApiRateLimit {
    max_requests: u32,
    window: Duration,
}

impl ApiRateLimit {
    pub fn new(max_requests: u32, window: Duration) -> Result<Self, ApiError> {
        if max_requests == 0 {
            return Err(ApiError::InvalidRateLimitConfig {
                key: "API_RATE_LIMIT_REQUESTS",
                value: max_requests.to_string(),
            });
        }
        if window.is_zero() {
            return Err(ApiError::InvalidRateLimitConfig {
                key: "API_RATE_LIMIT_WINDOW_SECONDS",
                value: String::from("0"),
            });
        }
        Ok(Self {
            max_requests,
            window,
        })
    }

    pub const fn max_requests(self) -> u32 {
        self.max_requests
    }

    pub const fn window(self) -> Duration {
        self.window
    }
}

impl Default for ApiRateLimit {
    fn default() -> Self {
        Self {
            max_requests: 600,
            window: Duration::from_secs(60),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ProcessRateLimiter {
    inner: Arc<Mutex<FixedWindow>>,
}

impl ProcessRateLimiter {
    pub(crate) fn new(limit: ApiRateLimit) -> Self {
        Self {
            inner: Arc::new(Mutex::new(FixedWindow::new(limit, Instant::now()))),
        }
    }

    fn allow(&self) -> bool {
        self.inner
            .lock()
            .is_ok_and(|mut limiter| limiter.allow(Instant::now()))
    }
}

#[derive(Debug)]
struct FixedWindow {
    limit: ApiRateLimit,
    started_at: Instant,
    used: u32,
}

impl FixedWindow {
    const fn new(limit: ApiRateLimit, started_at: Instant) -> Self {
        Self {
            limit,
            started_at,
            used: 0,
        }
    }

    fn allow(&mut self, now: Instant) -> bool {
        if now.duration_since(self.started_at) >= self.limit.window {
            self.started_at = now;
            self.used = 0;
        }
        if self.used >= self.limit.max_requests {
            return false;
        }
        self.used = self.used.saturating_add(1);
        true
    }
}

pub(crate) async fn enforce(
    State(limiter): State<ProcessRateLimiter>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if limiter.allow() {
        next.run(request).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            Json(RateLimitErrorResponse::new()),
        )
            .into_response()
    }
}

#[derive(Debug, Serialize)]
struct RateLimitErrorResponse {
    error: RateLimitErrorBody,
}

impl RateLimitErrorResponse {
    const fn new() -> Self {
        Self {
            error: RateLimitErrorBody {
                code: "rate_limited",
                message: "rate limit exceeded",
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct RateLimitErrorBody {
    code: &'static str,
    message: &'static str,
}
