use serde::{Deserialize, Serialize};

use crate::{EvidenceSourceKind, FilePath, TrustLevel};

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
    pub source_kind: EvidenceSourceKind,
    pub trust_level: TrustLevel,
}

impl EvidenceSpan {
    pub const fn new(file_path: FilePath, start: SourcePosition, end: SourcePosition) -> Self {
        Self::from_source(file_path, start, end, EvidenceSourceKind::RepositoryCode)
    }

    pub const fn from_source(
        file_path: FilePath,
        start: SourcePosition,
        end: SourcePosition,
        source_kind: EvidenceSourceKind,
    ) -> Self {
        Self {
            file_path,
            start,
            end,
            source_kind,
            trust_level: source_kind.default_trust_level(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UntrustedEvidence {
    text: String,
    source_kind: EvidenceSourceKind,
}

impl UntrustedEvidence {
    pub fn new(text: impl Into<String>) -> Self {
        Self::from_source(text, EvidenceSourceKind::RepositoryCode)
    }

    pub fn from_source(text: impl Into<String>, source_kind: EvidenceSourceKind) -> Self {
        Self {
            text: text.into(),
            source_kind,
        }
    }

    pub const fn trust_level(&self) -> TrustLevel {
        TrustLevel::Untrusted
    }

    pub const fn source_kind(&self) -> EvidenceSourceKind {
        self.source_kind
    }

    pub fn as_evidence_text(&self) -> &str {
        self.text.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TrustedInstructions {
    text: String,
    source_kind: EvidenceSourceKind,
}

impl TrustedInstructions {
    pub fn from_system(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            source_kind: EvidenceSourceKind::SystemInstruction,
        }
    }

    pub fn from_user(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            source_kind: EvidenceSourceKind::UserInstruction,
        }
    }

    pub const fn trust_level(&self) -> TrustLevel {
        TrustLevel::Trusted
    }

    pub const fn source_kind(&self) -> EvidenceSourceKind {
        self.source_kind
    }

    pub fn as_instruction_text(&self) -> &str {
        self.text.as_str()
    }
}
