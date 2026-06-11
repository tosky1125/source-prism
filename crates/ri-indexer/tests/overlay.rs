#![allow(
    missing_docs,
    reason = "Integration tests are executable contract names."
)]

use ri_core::{CoreError, FilePath};
use ri_indexer::{
    BaseFileRecord, OverlayEntry, OverlayFileStatus, OverlayMergedFile, merge_overlay,
};

#[test]
fn overlay_modified_file_shadows_base_record() -> Result<(), CoreError> {
    let base = vec![
        base_record("src/invoice.rs", "old-hash")?,
        base_record("src/refund.rs", "refund-hash")?,
    ];
    let overlay = vec![OverlayEntry::new(
        path("src/invoice.rs")?,
        OverlayFileStatus::Modified {
            content_hash: String::from("new-hash"),
        },
    )];

    let merged = merge_overlay(&base, &overlay);

    assert_eq!(
        merged,
        vec![
            OverlayMergedFile::Head {
                path: path("src/invoice.rs")?,
                content_hash: String::from("new-hash"),
            },
            OverlayMergedFile::Base(base_record("src/refund.rs", "refund-hash")?),
        ]
    );
    Ok(())
}

#[test]
fn overlay_deleted_file_removes_base_record() -> Result<(), CoreError> {
    let base = vec![
        base_record("src/invoice.rs", "old-hash")?,
        base_record("src/refund.rs", "refund-hash")?,
    ];
    let overlay = vec![OverlayEntry::new(
        path("src/invoice.rs")?,
        OverlayFileStatus::Deleted,
    )];

    let merged = merge_overlay(&base, &overlay);

    assert_eq!(
        merged,
        vec![OverlayMergedFile::Base(base_record(
            "src/refund.rs",
            "refund-hash"
        )?)]
    );
    Ok(())
}

#[test]
fn overlay_added_file_is_available_from_head() -> Result<(), CoreError> {
    let base = vec![base_record("src/refund.rs", "refund-hash")?];
    let overlay = vec![OverlayEntry::new(
        path("src/invoice.rs")?,
        OverlayFileStatus::Added {
            content_hash: String::from("new-hash"),
        },
    )];

    let merged = merge_overlay(&base, &overlay);

    assert_eq!(
        merged,
        vec![
            OverlayMergedFile::Head {
                path: path("src/invoice.rs")?,
                content_hash: String::from("new-hash"),
            },
            OverlayMergedFile::Base(base_record("src/refund.rs", "refund-hash")?),
        ]
    );
    Ok(())
}

#[test]
fn overlay_mode_only_change_preserves_content_hash() -> Result<(), CoreError> {
    let base = vec![base_record("src/invoice.rs", "old-hash")?];
    let overlay = vec![OverlayEntry::new(
        path("src/invoice.rs")?,
        OverlayFileStatus::ModeOnly,
    )];

    let merged = merge_overlay(&base, &overlay);

    assert_eq!(
        merged,
        vec![OverlayMergedFile::Head {
            path: path("src/invoice.rs")?,
            content_hash: String::from("old-hash"),
        }]
    );
    Ok(())
}

#[test]
fn overlay_renamed_file_removes_old_path_and_adds_new_path() -> Result<(), CoreError> {
    let base = vec![
        base_record("src/invoice_old.rs", "old-hash")?,
        base_record("src/refund.rs", "refund-hash")?,
    ];
    let overlay = vec![OverlayEntry::new(
        path("src/invoice_new.rs")?,
        OverlayFileStatus::Renamed {
            previous_path: path("src/invoice_old.rs")?,
            content_hash: String::from("new-hash"),
        },
    )];

    let merged = merge_overlay(&base, &overlay);

    assert_eq!(
        merged,
        vec![
            OverlayMergedFile::Head {
                path: path("src/invoice_new.rs")?,
                content_hash: String::from("new-hash"),
            },
            OverlayMergedFile::Base(base_record("src/refund.rs", "refund-hash")?),
        ]
    );
    Ok(())
}

fn base_record(file_path: &str, content_hash: &str) -> Result<BaseFileRecord, CoreError> {
    Ok(BaseFileRecord::new(
        path(file_path)?,
        String::from(content_hash),
    ))
}

fn path(value: &str) -> Result<FilePath, CoreError> {
    FilePath::new(value)
}
