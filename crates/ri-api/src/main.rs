#![allow(missing_docs, reason = "Binary entry point delegates to library API.")]
#![allow(
    clippy::multiple_crate_versions,
    reason = "SQLx and Reqwest TLS dependencies pull duplicate platform crates outside this crate's control."
)]

use ri_api::{ApiError, app_with_rate_limit, bind_addr, rate_limit_from_env, state_from_env};

#[tokio::main]
async fn main() -> Result<(), ApiError> {
    let listener = tokio::net::TcpListener::bind(bind_addr()?).await?;
    axum::serve(
        listener,
        app_with_rate_limit(state_from_env()?, rate_limit_from_env()?),
    )
    .await?;
    Ok(())
}
