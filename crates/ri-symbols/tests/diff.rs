#![allow(missing_docs, reason = "Integration test names document behavior.")]

use ri_symbols::{ChangedFileStatus, parse_changed_files};

#[test]
fn parse_changed_files_reports_overlay_statuses_from_git_diff()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a git diff with added, modified, deleted, renamed, and mode-only files.
    let diff = "\
diff --git a/src/lib.rs b/src/lib.rs
index 1111111..2222222 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -1,1 +1,2 @@
+pub fn changed() {}
diff --git a/src/new.rs b/src/new.rs
new file mode 100644
--- /dev/null
+++ b/src/new.rs
@@ -0,0 +1 @@
+pub fn new_file() {}
diff --git a/src/deleted.rs b/src/deleted.rs
deleted file mode 100644
--- a/src/deleted.rs
+++ /dev/null
@@ -1 +0,0 @@
-pub fn deleted() {}
diff --git a/src/old.rs b/src/renamed.rs
similarity index 92%
rename from src/old.rs
rename to src/renamed.rs
diff --git a/src/mode.rs b/src/mode.rs
old mode 100644
new mode 100755
";

    // When: changed files are parsed from the diff.
    let changed_files = parse_changed_files(diff);

    // Then: each file has the overlay status needed by changed-file indexing.
    assert_eq!(changed_files.len(), 5);
    let modified = changed_files.first().ok_or("missing modified file")?;
    let added = changed_files.get(1).ok_or("missing added file")?;
    let deleted = changed_files.get(2).ok_or("missing deleted file")?;
    let renamed = changed_files.get(3).ok_or("missing renamed file")?;
    let mode_only = changed_files.get(4).ok_or("missing mode-only file")?;
    assert_eq!(modified.path, "src/lib.rs");
    assert_eq!(modified.status, ChangedFileStatus::Modified);
    assert_eq!(added.status, ChangedFileStatus::Added);
    assert_eq!(deleted.status, ChangedFileStatus::Deleted);
    assert_eq!(renamed.path, "src/renamed.rs");
    assert_eq!(renamed.previous_path.as_deref(), Some("src/old.rs"));
    assert_eq!(renamed.status, ChangedFileStatus::Renamed);
    assert_eq!(mode_only.status, ChangedFileStatus::ModeOnly);
    Ok(())
}
