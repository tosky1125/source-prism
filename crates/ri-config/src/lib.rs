#![allow(
    missing_docs,
    reason = "Config contract names are self-describing at this milestone."
)]

use std::{
    collections::BTreeMap,
    fs,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use thiserror::Error;
use url::Url;

const DATABASE_URL: &str = "DATABASE_URL";
const OPENSEARCH_URL: &str = "OPENSEARCH_URL";
const API_BIND_ADDR: &str = "API_BIND_ADDR";
const API_RATE_LIMIT_REQUESTS: &str = "API_RATE_LIMIT_REQUESTS";
const API_RATE_LIMIT_WINDOW_SECONDS: &str = "API_RATE_LIMIT_WINDOW_SECONDS";
const DEFAULT_API_BIND_ADDR: &str = "127.0.0.1:3000";
const DEFAULT_API_RATE_LIMIT_REQUESTS: u32 = 600;
const DEFAULT_API_RATE_LIMIT_WINDOW_SECONDS: u64 = 60;

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct RuntimeConfig {
    pub database: DatabaseConfig,
    pub opensearch: OpenSearchConfig,
    pub api: ApiConfig,
    pub worker: WorkerConfig,
    pub evidence_dir: PathBuf,
}

impl RuntimeConfig {
    pub fn from_env_map(env: &BTreeMap<String, String>) -> Result<Self, ConfigError> {
        let database_url = required(env, DATABASE_URL)?;
        let opensearch_url = required(env, OPENSEARCH_URL)?;
        let bind_addr = optional(env, API_BIND_ADDR).unwrap_or(DEFAULT_API_BIND_ADDR);
        let rate_limit_requests = optional_positive_u32(
            env,
            API_RATE_LIMIT_REQUESTS,
            DEFAULT_API_RATE_LIMIT_REQUESTS,
        )?;
        let rate_limit_window_seconds = optional_positive_u64(
            env,
            API_RATE_LIMIT_WINDOW_SECONDS,
            DEFAULT_API_RATE_LIMIT_WINDOW_SECONDS,
        )?;

        let bind_addr: SocketAddr = bind_addr
            .parse()
            .map_err(|_| ConfigError::InvalidBindAddress)?;
        if !bind_addr.ip().is_loopback() {
            return Err(ConfigError::PublicApiBindAddress { bind_addr });
        }

        Ok(Self {
            database: DatabaseConfig {
                url: parse_url(DATABASE_URL, database_url)?,
            },
            opensearch: OpenSearchConfig {
                url: parse_url(OPENSEARCH_URL, opensearch_url)?,
            },
            api: ApiConfig {
                bind_addr,
                rate_limit_requests,
                rate_limit_window_seconds,
            },
            worker: WorkerConfig { concurrency: 1 },
            evidence_dir: PathBuf::from(".omo/evidence"),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct DatabaseConfig {
    pub url: Url,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct OpenSearchConfig {
    pub url: Url,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ApiConfig {
    pub bind_addr: SocketAddr,
    pub rate_limit_requests: u32,
    pub rate_limit_window_seconds: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct WorkerConfig {
    pub concurrency: u16,
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ConfigError {
    #[error("missing required config: {key}")]
    MissingEnv { key: &'static str },
    #[error("invalid URL for config key: {key}")]
    InvalidUrl { key: &'static str },
    #[error("invalid API bind address")]
    InvalidBindAddress,
    #[error("public API bind address requires auth/tenancy first: {bind_addr}")]
    PublicApiBindAddress { bind_addr: SocketAddr },
    #[error("invalid positive integer for config key: {key}")]
    InvalidPositiveInteger { key: &'static str },
    #[error("failed to read config env file: {}", path.display())]
    EnvFileRead {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid config env line: {line}")]
    InvalidEnvLine { line: usize },
}

pub fn load_env_file(path: &Path) -> Result<BTreeMap<String, String>, ConfigError> {
    let content = fs::read_to_string(path).map_err(|source| ConfigError::EnvFileRead {
        path: path.to_path_buf(),
        source,
    })?;
    parse_env_content(&content)
}

pub fn redact_value(key: &str, value: &str) -> String {
    let normalized = key.to_ascii_uppercase();
    if normalized.contains("PASSWORD")
        || normalized.contains("SECRET")
        || normalized.contains("TOKEN")
        || normalized.ends_with("_KEY")
    {
        String::from("[redacted]")
    } else {
        String::from(value)
    }
}

fn parse_env_content(content: &str) -> Result<BTreeMap<String, String>, ConfigError> {
    let mut values = BTreeMap::new();
    for (index, raw_line) in content.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(ConfigError::InvalidEnvLine {
                line: index.saturating_add(1),
            });
        };
        values.insert(String::from(key.trim()), String::from(value.trim()));
    }
    Ok(values)
}

fn required<'a>(
    env: &'a BTreeMap<String, String>,
    key: &'static str,
) -> Result<&'a str, ConfigError> {
    optional(env, key).ok_or(ConfigError::MissingEnv { key })
}

fn optional<'a>(env: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    env.get(key)
        .map(String::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn parse_url(key: &'static str, value: &str) -> Result<Url, ConfigError> {
    Url::parse(value).map_err(|_| ConfigError::InvalidUrl { key })
}

fn optional_positive_u32(
    env: &BTreeMap<String, String>,
    key: &'static str,
    default: u32,
) -> Result<u32, ConfigError> {
    optional(env, key).map_or(Ok(default), |value| {
        value
            .parse::<u32>()
            .ok()
            .filter(|parsed| *parsed > 0)
            .ok_or(ConfigError::InvalidPositiveInteger { key })
    })
}

fn optional_positive_u64(
    env: &BTreeMap<String, String>,
    key: &'static str,
    default: u64,
) -> Result<u64, ConfigError> {
    optional(env, key).map_or(Ok(default), |value| {
        value
            .parse::<u64>()
            .ok()
            .filter(|parsed| *parsed > 0)
            .ok_or(ConfigError::InvalidPositiveInteger { key })
    })
}
