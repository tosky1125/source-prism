#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_api::{ApiError, parse_bind_addr};

#[test]
fn parse_bind_addr_rejects_public_bind_without_auth_tenancy() {
    // Given: a public bind address while Source Prism has no auth/tenancy mode.
    let value = "0.0.0.0:4096";

    // When: the API parses its bind address.
    let error = parse_bind_addr(value).expect_err("public bind rejected");

    // Then: startup is stopped before exposing unauthenticated repo evidence.
    assert!(matches!(error, ApiError::PublicBindAddress { .. }));
    assert!(error.to_string().contains("auth/tenancy"));
}

#[test]
fn parse_bind_addr_accepts_loopback_bind() -> Result<(), Box<dyn std::error::Error>> {
    // Given: a loopback-only bind address for local development.
    let value = "127.0.0.1:4096";

    // When: the API parses its bind address.
    let addr = parse_bind_addr(value)?;

    // Then: local-only startup remains available.
    assert_eq!(addr.to_string(), value);
    Ok(())
}
