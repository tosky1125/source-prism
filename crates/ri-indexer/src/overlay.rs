use std::collections::BTreeMap;

use ri_core::FilePath;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseFileRecord {
    path: FilePath,
    content_hash: String,
}

impl BaseFileRecord {
    pub const fn new(path: FilePath, content_hash: String) -> Self {
        Self { path, content_hash }
    }

    pub const fn path(&self) -> &FilePath {
        &self.path
    }

    pub fn content_hash(&self) -> &str {
        self.content_hash.as_str()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OverlayEntry {
    path: FilePath,
    status: OverlayFileStatus,
}

impl OverlayEntry {
    pub const fn new(path: FilePath, status: OverlayFileStatus) -> Self {
        Self { path, status }
    }

    pub const fn path(&self) -> &FilePath {
        &self.path
    }

    pub const fn status(&self) -> &OverlayFileStatus {
        &self.status
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OverlayFileStatus {
    Added {
        content_hash: String,
    },
    Modified {
        content_hash: String,
    },
    Deleted,
    Renamed {
        previous_path: FilePath,
        content_hash: String,
    },
    ModeOnly,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum OverlayMergedFile {
    Base(BaseFileRecord),
    Head {
        path: FilePath,
        content_hash: String,
    },
}

pub fn merge_overlay(
    base_records: &[BaseFileRecord],
    overlay_entries: &[OverlayEntry],
) -> Vec<OverlayMergedFile> {
    let mut merged = BTreeMap::new();

    for base in base_records {
        merged.insert(base.path.clone(), OverlayMergedFile::Base(base.clone()));
    }

    for entry in overlay_entries {
        match entry.status() {
            OverlayFileStatus::Added { content_hash }
            | OverlayFileStatus::Modified { content_hash } => {
                merged.insert(
                    entry.path().clone(),
                    OverlayMergedFile::Head {
                        path: entry.path().clone(),
                        content_hash: content_hash.clone(),
                    },
                );
            }
            OverlayFileStatus::Deleted => {
                merged.remove(entry.path());
            }
            OverlayFileStatus::Renamed {
                previous_path,
                content_hash,
            } => {
                merged.remove(previous_path);
                merged.insert(
                    entry.path().clone(),
                    OverlayMergedFile::Head {
                        path: entry.path().clone(),
                        content_hash: content_hash.clone(),
                    },
                );
            }
            OverlayFileStatus::ModeOnly => {
                if let Some(OverlayMergedFile::Base(base)) = merged.remove(entry.path()) {
                    merged.insert(
                        entry.path().clone(),
                        OverlayMergedFile::Head {
                            path: entry.path().clone(),
                            content_hash: String::from(base.content_hash()),
                        },
                    );
                }
            }
        }
    }

    merged.into_values().collect()
}
