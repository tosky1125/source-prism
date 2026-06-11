use serde::{Deserialize, Serialize};

use crate::{FilePath, TrustLevel};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SourcePosition {
    pub line: u32,
    pub column: u32,
}

impl SourcePosition {
    pub const fn new(line: u32, column: u32) -> Self {
        Self { line, column }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct EvidenceSpan {
    pub file_path: FilePath,
    pub start: SourcePosition,
    pub end: SourcePosition,
    pub trust_level: TrustLevel,
}

impl EvidenceSpan {
    pub const fn new(file_path: FilePath, start: SourcePosition, end: SourcePosition) -> Self {
        Self {
            file_path,
            start,
            end,
            trust_level: TrustLevel::Untrusted,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UntrustedEvidence {
    text: String,
}

impl UntrustedEvidence {
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    pub const fn trust_level(&self) -> TrustLevel {
        TrustLevel::Untrusted
    }

    pub fn as_evidence_text(&self) -> &str {
        self.text.as_str()
    }
}
