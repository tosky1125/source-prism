use serde::{Deserialize, Serialize};

use crate::ChangedLine;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ChangedFileStatus {
    Added,
    Modified,
    Deleted,
    Renamed,
    ModeOnly,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ChangedFile {
    pub path: String,
    pub previous_path: Option<String>,
    pub status: ChangedFileStatus,
}

impl ChangedFile {
    fn new(
        path: impl Into<String>,
        previous_path: Option<String>,
        status: ChangedFileStatus,
    ) -> Self {
        Self {
            path: path.into(),
            previous_path,
            status,
        }
    }
}

pub fn parse_changed_lines(diff: &str) -> Vec<ChangedLine> {
    let mut file_path = None::<String>;
    let mut new_line = None::<u32>;
    let mut changed = Vec::new();

    for line in diff.lines() {
        if let Some(path) = line.strip_prefix("+++ ") {
            file_path = parse_diff_path(path);
            continue;
        }
        if let Some(header) = line.strip_prefix("@@") {
            new_line = parse_hunk_new_start(header);
            continue;
        }
        let Some(current_line) = new_line else {
            continue;
        };
        if line.starts_with('+') {
            if let Some(path) = &file_path {
                changed.push(ChangedLine::new(path.clone(), current_line));
            }
            new_line = current_line.checked_add(1);
        } else if !line.starts_with('-') && !line.starts_with('\\') {
            new_line = current_line.checked_add(1);
        }
    }
    changed
}

pub fn parse_changed_files(diff: &str) -> Vec<ChangedFile> {
    let mut files = Vec::new();
    let mut current = None::<DiffFile>;

    for line in diff.lines() {
        if let Some((old_path, new_path)) = parse_diff_git(line) {
            finish_current(&mut files, current.take());
            current = Some(DiffFile::new(old_path, new_path));
            continue;
        }
        let Some(file) = current.as_mut() else {
            continue;
        };
        file.apply_line(line);
    }

    finish_current(&mut files, current);
    files
}

#[derive(Debug)]
struct DiffFile {
    old_path: Option<String>,
    new_path: Option<String>,
    old_content_path: Option<String>,
    new_content_path: Option<String>,
    rename_from: Option<String>,
    rename_to: Option<String>,
    evidence: DiffEvidence,
}

impl DiffFile {
    const fn new(old_path: String, new_path: String) -> Self {
        Self {
            old_path: Some(old_path),
            new_path: Some(new_path),
            old_content_path: None,
            new_content_path: None,
            rename_from: None,
            rename_to: None,
            evidence: DiffEvidence::Unknown,
        }
    }

    fn apply_line(&mut self, line: &str) {
        if line.starts_with("@@") {
            if matches!(
                self.evidence,
                DiffEvidence::Unknown | DiffEvidence::ModeOnly
            ) {
                self.evidence = DiffEvidence::Modified;
            }
        } else if line.starts_with("new file mode ") {
            self.evidence = DiffEvidence::Added;
        } else if line.starts_with("deleted file mode ") {
            self.evidence = DiffEvidence::Deleted;
        } else if line.starts_with("old mode ") || line.starts_with("new mode ") {
            if self.evidence == DiffEvidence::Unknown {
                self.evidence = DiffEvidence::ModeOnly;
            }
        } else if let Some(path) = line.strip_prefix("rename from ") {
            self.rename_from = Some(path.to_owned());
        } else if let Some(path) = line.strip_prefix("rename to ") {
            self.rename_to = Some(path.to_owned());
        } else if let Some(path) = line.strip_prefix("--- ") {
            self.old_content_path = parse_diff_path(path);
        } else if let Some(path) = line.strip_prefix("+++ ") {
            self.new_content_path = parse_diff_path(path);
        }
    }

    fn into_changed_file(self) -> Option<ChangedFile> {
        let status = self.status();
        let path = self.output_path(status)?;
        let previous_path = match status {
            ChangedFileStatus::Renamed => self.rename_from.or(self.old_path),
            ChangedFileStatus::Added
            | ChangedFileStatus::Modified
            | ChangedFileStatus::Deleted
            | ChangedFileStatus::ModeOnly => None,
        };
        Some(ChangedFile::new(path, previous_path, status))
    }

    const fn status(&self) -> ChangedFileStatus {
        if self.rename_to.is_some() {
            ChangedFileStatus::Renamed
        } else {
            match self.evidence {
                DiffEvidence::Added => ChangedFileStatus::Added,
                DiffEvidence::Deleted => ChangedFileStatus::Deleted,
                DiffEvidence::ModeOnly => ChangedFileStatus::ModeOnly,
                DiffEvidence::Modified | DiffEvidence::Unknown => ChangedFileStatus::Modified,
            }
        }
    }

    fn output_path(&self, status: ChangedFileStatus) -> Option<String> {
        match status {
            ChangedFileStatus::Added
            | ChangedFileStatus::Modified
            | ChangedFileStatus::ModeOnly => self
                .new_content_path
                .clone()
                .or_else(|| self.new_path.clone()),
            ChangedFileStatus::Deleted => self
                .old_content_path
                .clone()
                .or_else(|| self.old_path.clone()),
            ChangedFileStatus::Renamed => self.rename_to.clone().or_else(|| self.new_path.clone()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiffEvidence {
    Unknown,
    Added,
    Deleted,
    ModeOnly,
    Modified,
}

fn finish_current(files: &mut Vec<ChangedFile>, current: Option<DiffFile>) {
    if let Some(file) = current.and_then(DiffFile::into_changed_file) {
        files.push(file);
    }
}

fn parse_diff_git(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("diff --git ")?;
    let mut parts = rest.split_whitespace();
    let old_path = parse_diff_path(parts.next()?)?;
    let new_path = parse_diff_path(parts.next()?)?;
    Some((old_path, new_path))
}

fn parse_diff_path(path: &str) -> Option<String> {
    if path == "/dev/null" {
        return None;
    }
    Some(
        path.strip_prefix("a/")
            .or_else(|| path.strip_prefix("b/"))
            .unwrap_or(path)
            .to_owned(),
    )
}

fn parse_hunk_new_start(header: &str) -> Option<u32> {
    header
        .split_whitespace()
        .find_map(|part| part.strip_prefix('+'))
        .and_then(|part| part.split(',').next())
        .and_then(|line| line.parse::<u32>().ok())
}
