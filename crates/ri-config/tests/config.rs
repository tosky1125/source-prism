#![allow(
    missing_docs,
    reason = "Integration tests are executable contract names."
)]

use std::collections::BTreeMap;

use ri_config::{ConfigError, RuntimeConfig, load_env_file, redact_value};

#[test]
fn local_only_api_bind_is_default() {
    let config = RuntimeConfig::from_env_map(&required_env()).expect("config");

    assert_eq!(config.api.bind_addr.to_string(), "127.0.0.1:3000");
}

#[test]
fn public_api_bind_is_rejected_without_auth_tenancy() {
    let mut env = required_env();
    env.insert(String::from("API_BIND_ADDR"), String::from("0.0.0.0:3000"));

    let error = RuntimeConfig::from_env_map(&env).expect_err("public bind rejected");

    assert!(matches!(error, ConfigError::PublicApiBindAddress { .. }));
    assert!(error.to_string().contains("auth/tenancy"));
}

#[test]
fn missing_required_env_is_rejected() {
    let error = RuntimeConfig::from_env_map(&BTreeMap::new()).expect_err("missing env");

    assert!(matches!(
        error,
        ConfigError::MissingEnv {
            key: "DATABASE_URL"
        }
    ));
    assert!(!error.to_string().contains("password"));
    assert!(!error.to_string().contains("secret"));
    assert!(!error.to_string().contains("token"));
}

#[test]
fn invalid_urls_are_rejected() {
    let mut env = required_env();
    env.insert(String::from("OPENSEARCH_URL"), String::from("not a url"));

    let error = RuntimeConfig::from_env_map(&env).expect_err("invalid url");

    assert!(matches!(
        error,
        ConfigError::InvalidUrl {
            key: "OPENSEARCH_URL"
        }
    ));
}

#[test]
fn redaction_is_deterministic_for_secret_looking_values() {
    assert_eq!(redact_value("GITHUB_TOKEN", "abc123"), "[redacted]");
    assert_eq!(redact_value("DATABASE_PASSWORD", "abc123"), "[redacted]");
    assert_eq!(redact_value("PLAIN_SETTING", "abc123"), "abc123");
    assert_eq!(redact_value("GITHUB_TOKEN", "different"), "[redacted]");
}

#[test]
fn env_file_loader_reads_key_value_pairs() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("source-prism.env");
    std::fs::write(
        &path,
        "DATABASE_URL=postgres://source_prism:source_prism@localhost:5432/source_prism\nOPENSEARCH_URL=http://localhost:9200\n",
    )
    .expect("write env file");

    let config =
        RuntimeConfig::from_env_map(&load_env_file(&path).expect("load env file")).expect("config");

    assert_eq!(
        config.database.url.as_str(),
        "postgres://source_prism:source_prism@localhost:5432/source_prism"
    );
    assert_eq!(config.opensearch.url.as_str(), "http://localhost:9200/");
}

fn required_env() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            String::from("DATABASE_URL"),
            String::from("postgres://source_prism:source_prism@localhost:5432/source_prism"),
        ),
        (
            String::from("OPENSEARCH_URL"),
            String::from("http://localhost:9200"),
        ),
    ])
}
