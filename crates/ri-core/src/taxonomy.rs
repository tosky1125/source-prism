use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Language {
    TypeScript,
    JavaScript,
    Php,
    Python,
    Java,
    Go,
    Rust,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SymbolKind {
    Module,
    Class,
    Interface,
    Enum,
    Function,
    Method,
    Constructor,
    Field,
    RouteHandler,
    TestCase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EdgeKind {
    Contains,
    Imports,
    Calls,
    References,
    Extends,
    Implements,
    Overrides,
    RouteToHandler,
    TestCovers,
    ReadsTable,
    WritesTable,
    PublishesEvent,
    ConsumesEvent,
    UsesEnv,
    UsesFeatureFlag,
}

impl EdgeKind {
    pub(crate) const fn as_id_part(self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::Imports => "imports",
            Self::Calls => "calls",
            Self::References => "references",
            Self::Extends => "extends",
            Self::Implements => "implements",
            Self::Overrides => "overrides",
            Self::RouteToHandler => "route_to_handler",
            Self::TestCovers => "test_covers",
            Self::ReadsTable => "reads_table",
            Self::WritesTable => "writes_table",
            Self::PublishesEvent => "publishes_event",
            Self::ConsumesEvent => "consumes_event",
            Self::UsesEnv => "uses_env",
            Self::UsesFeatureFlag => "uses_feature_flag",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum Confidence {
    Exact,
    High,
    Medium,
    Low,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum TrustLevel {
    Trusted,
    Untrusted,
}
