#![allow(
    missing_docs,
    reason = "The manifest API names are self-describing at this milestone."
)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "gix currently pulls duplicate transitive crate versions."
)]

use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};

use ri_core::{CoreError, FilePath, Language};
use sha2::{Digest, Sha256};
use thiserror::Error;

mod classification;

use classification::{guess_language, is_generated_path, is_test_path, is_vendor_path};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalManifest {
    files: Vec<FileManifest>,
}

impl LocalManifest {
    pub fn extract(path: impl AsRef<Path>) -> Result<Self, GitError> {
        let repo = gix::discover(path).map_err(Box::new)?;
        let worktree = repo.workdir().ok_or(GitError::BareRepository)?;
        let mut files = Vec::new();
        collect_files(worktree, worktree, &mut files)?;
        files.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(Self { files })
    }

    pub fn files(&self) -> &[FileManifest] {
        self.files.as_slice()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileManifest {
    path: FilePath,
    language: Language,
    size_bytes: u64,
    content_sha256: String,
    is_generated: bool,
    is_vendor: bool,
    is_test: bool,
}

impl FileManifest {
    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    pub const fn language(&self) -> Language {
        self.language
    }

    pub const fn size_bytes(&self) -> u64 {
        self.size_bytes
    }

    pub fn content_sha256(&self) -> &str {
        self.content_sha256.as_str()
    }

    pub const fn is_generated(&self) -> bool {
        self.is_generated
    }

    pub const fn is_vendor(&self) -> bool {
        self.is_vendor
    }

    pub const fn is_test(&self) -> bool {
        self.is_test
    }
}

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GitError {
    #[error("could not discover git repository")]
    Discover(#[from] Box<gix::discover::Error>),
    #[error("repository has no worktree")]
    BareRepository,
    #[error("repository path is not valid UTF-8: {path}")]
    PathEncoding { path: PathBuf },
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("could not read {path}: {source}")]
    Io { path: PathBuf, source: io::Error },
    #[error("read length exceeded buffer size for {path}")]
    InvalidReadLength { path: PathBuf },
}

fn collect_files(root: &Path, dir: &Path, files: &mut Vec<FileManifest>) -> Result<(), GitError> {
    for entry in read_dir(dir)? {
        let entry = entry.map_err(|source| GitError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|source| GitError::Io {
            path: path.clone(),
            source,
        })?;

        if file_type.is_dir() {
            if should_enter_dir(root, &path)? {
                collect_files(root, &path, files)?;
            }
            continue;
        }

        if file_type.is_file() {
            if let Some(record) = manifest_file(root, &path)? {
                files.push(record);
            }
        }
    }
    Ok(())
}

fn read_dir(path: &Path) -> Result<fs::ReadDir, GitError> {
    fs::read_dir(path).map_err(|source| GitError::Io {
        path: path.to_path_buf(),
        source,
    })
}

fn should_enter_dir(root: &Path, path: &Path) -> Result<bool, GitError> {
    let relative_path = relative_path(root, path)?;
    Ok(!is_ignored_local_artifact_path(&relative_path))
}

fn manifest_file(root: &Path, path: &Path) -> Result<Option<FileManifest>, GitError> {
    let relative_path = relative_path(root, path)?;
    if is_ignored_local_artifact_path(&relative_path) {
        return Ok(None);
    }
    let metadata = fs::metadata(path).map_err(|source| GitError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let hash = content_hash(path)?;
    let Some(content_sha256) = hash else {
        return Ok(None);
    };

    Ok(Some(FileManifest {
        language: guess_language(&relative_path),
        is_generated: is_generated_path(&relative_path),
        is_vendor: is_vendor_path(&relative_path),
        is_test: is_test_path(&relative_path),
        path: FilePath::new(&relative_path)?,
        size_bytes: metadata.len(),
        content_sha256,
    }))
}

fn content_hash(path: &Path) -> Result<Option<String>, GitError> {
    let mut file = File::open(path).map_err(|source| GitError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8 * 1024];
    loop {
        let bytes_read = file.read(&mut buffer).map_err(|source| GitError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        if bytes_read == 0 {
            return Ok(Some(format!("{:x}", hasher.finalize())));
        }
        let chunk = buffer
            .get(..bytes_read)
            .ok_or_else(|| GitError::InvalidReadLength {
                path: path.to_path_buf(),
            })?;
        if chunk.contains(&0) {
            return Ok(None);
        }
        hasher.update(chunk);
    }
}

fn relative_path(root: &Path, path: &Path) -> Result<String, GitError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| GitError::PathEncoding {
            path: path.to_path_buf(),
        })?;
    let mut parts = Vec::new();
    for component in relative.components() {
        if let Some(part) = component_str(path, component)? {
            parts.push(part);
        }
    }
    Ok(parts.join("/"))
}

fn component_str<'component>(
    path: &Path,
    component: Component<'component>,
) -> Result<Option<&'component str>, GitError> {
    match component {
        Component::Normal(value) => {
            value
                .to_str()
                .map(Some)
                .ok_or_else(|| GitError::PathEncoding {
                    path: path.to_path_buf(),
                })
        }
        Component::CurDir | Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
            Ok(None)
        }
    }
}

fn is_git_internal_path(path: &str) -> bool {
    path == ".git" || path.starts_with(".git/")
}

fn is_ignored_local_artifact_path(path: &str) -> bool {
    is_git_internal_path(path)
        || path == "target"
        || path.starts_with("target/")
        || path == ".omo/evidence"
        || path.starts_with(".omo/evidence/")
}
