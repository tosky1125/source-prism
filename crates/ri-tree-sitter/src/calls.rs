#![allow(
    clippy::redundant_pub_crate,
    reason = "Parent module consumes this private-module extraction entry point."
)]

use ri_core::Language;
use ri_parser::{CallReference, SourceFile};
use ri_symbols::SymbolRange;
use tree_sitter::Node;

use crate::names::node_text;

const RUST_CALL_EXPRESSION: &str = "call_expression";

pub(crate) fn extract_tree_calls(file: &SourceFile<'_>, root: Node<'_>) -> Vec<CallReference> {
    let mut calls = Vec::new();
    walk(file, root, &mut calls);
    calls
}

fn walk(file: &SourceFile<'_>, node: Node<'_>, calls: &mut Vec<CallReference>) {
    if let Some(call) = call_for_node(file, node) {
        calls.push(call);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(file, child, calls);
    }
}

fn call_for_node(file: &SourceFile<'_>, node: Node<'_>) -> Option<CallReference> {
    if file.language != Language::Rust || node.kind() != RUST_CALL_EXPRESSION {
        return None;
    }
    let function = node.child_by_field_name("function")?;
    let target_name = rust_target_name(&node_text(file.source, function)?)?;
    Some(CallReference::new(
        file.path.clone(),
        file.language,
        target_name,
        range_for(node),
    ))
}

fn rust_target_name(raw: &str) -> Option<String> {
    let last_segment = raw.rsplit("::").next()?.rsplit('.').next()?.trim();
    let target = last_segment
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect::<String>();
    if target.is_empty() {
        None
    } else {
        Some(target)
    }
}

fn range_for(node: Node<'_>) -> SymbolRange {
    let start = node.start_position();
    let end = node.end_position();
    SymbolRange::new(
        u32::try_from(start.row.saturating_add(1)).unwrap_or(u32::MAX),
        u32::try_from(start.column).unwrap_or(u32::MAX),
        u32::try_from(end.row.saturating_add(1)).unwrap_or(u32::MAX),
        u32::try_from(end.column).unwrap_or(u32::MAX),
    )
}
