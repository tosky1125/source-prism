#![allow(missing_docs, reason = "Milestone scaffold exposes no public API yet.")]

mod overlay;

pub use overlay::{
    BaseFileRecord, OverlayEntry, OverlayFileStatus, OverlayMergedFile, merge_overlay,
};
