use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

struct LocalTempRepo {
    path: PathBuf,
}

impl LocalTempRepo {
    fn create(prefix: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!("{prefix}-{suffix}"));
        fs::create_dir_all(path.join("src"))?;
        run_git(&path, ["init"])?;
        run_git(
            &path,
            ["config", "user.email", "source-prism@example.invalid"],
        )?;
        run_git(&path, ["config", "user.name", "Source Prism Test"])?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        self.path.as_path()
    }

    fn write_file(&self, path: &str, body: &str) -> Result<(), std::io::Error> {
        fs::write(self.path.join(path), body)
    }

    fn commit(&self) -> Result<(), Box<dyn std::error::Error>> {
        run_git(&self.path, ["add", "."])?;
        run_git(&self.path, ["commit", "-m", "fixture"])?;
        Ok(())
    }

    fn cleanup(&self) -> Result<(), std::io::Error> {
        fs::remove_dir_all(&self.path)
    }
}

fn run_git<const N: usize>(
    path: &Path,
    args: [&str; N],
) -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("git").current_dir(path).args(args).output()?;
    if output.status.success() {
        return Ok(());
    }
    Err(std::io::Error::other(String::from_utf8_lossy(&output.stderr).to_string()).into())
}
