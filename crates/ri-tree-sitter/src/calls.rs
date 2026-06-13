#![allow(
    clippy::redundant_pub_crate,
    reason = "Parent module consumes this private-module extraction entry point."
)]

use ri_core::Language;
use ri_parser::{CallReference, SourceFile};
use ri_symbols::SymbolRange;
use std::collections::BTreeSet;
use tree_sitter::Node;

use crate::names::node_text;

const RUST_CALL_EXPRESSION: &str = "call_expression";
const RUST_MACRO_INVOCATION: &str = "macro_invocation";
const IDENTIFIER: &str = "identifier";

pub(crate) fn extract_tree_calls(file: &SourceFile<'_>, root: Node<'_>) -> Vec<CallReference> {
    let mut calls = Vec::new();
    walk(file, root, &mut calls);
    calls
}

fn walk(file: &SourceFile<'_>, node: Node<'_>, calls: &mut Vec<CallReference>) {
    calls.extend(calls_for_node(file, node));
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(file, child, calls);
    }
}

fn calls_for_node(file: &SourceFile<'_>, node: Node<'_>) -> Vec<CallReference> {
    if node.kind() == RUST_CALL_EXPRESSION {
        return match file.language {
            Language::Rust => rust_call_expression(file, node).into_iter().collect(),
            Language::TypeScript | Language::JavaScript => {
                identifier_call_expression(file, node).into_iter().collect()
            }
            _ => Vec::new(),
        };
    }
    if file.language == Language::Rust && node.kind() == RUST_MACRO_INVOCATION {
        return macro_calls(file, node);
    }
    Vec::new()
}

fn rust_call_expression(file: &SourceFile<'_>, node: Node<'_>) -> Option<CallReference> {
    let function = node.child_by_field_name("function")?;
    let target_name = rust_target_name(&node_text(file.source, function)?)?;
    Some(CallReference::new(
        file.path.clone(),
        file.language,
        target_name,
        range_for(node),
    ))
}

fn identifier_call_expression(file: &SourceFile<'_>, node: Node<'_>) -> Option<CallReference> {
    let function = node.child_by_field_name("function")?;
    if function.kind() != IDENTIFIER {
        return None;
    }
    let target_name = node_text(file.source, function)?;
    Some(CallReference::new(
        file.path.clone(),
        file.language,
        target_name,
        range_for(node),
    ))
}

fn macro_calls(file: &SourceFile<'_>, node: Node<'_>) -> Vec<CallReference> {
    let Some(raw) = node_text(file.source, node) else {
        return Vec::new();
    };
    rust_macro_target_names(&raw)
        .into_iter()
        .map(|target_name| {
            CallReference::new(
                file.path.clone(),
                file.language,
                target_name,
                range_for(node),
            )
        })
        .collect()
}

fn rust_target_name(raw: &str) -> Option<String> {
    if raw.contains('.') {
        return None;
    }
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

fn rust_macro_target_names(raw: &str) -> BTreeSet<String> {
    raw.match_indices('(')
        .filter_map(|(index, _)| rust_target_before_paren(&raw[..index]))
        .collect()
}

fn rust_target_before_paren(prefix: &str) -> Option<String> {
    let trimmed = prefix.trim_end();
    let end = trimmed.len();
    let start = trimmed
        .char_indices()
        .rev()
        .find(|(_, character)| {
            !character.is_ascii_alphanumeric() && *character != '_' && *character != ':'
        })
        .map_or(0, |(index, character)| {
            index.saturating_add(character.len_utf8())
        });
    rust_target_name(&trimmed[start..end])
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
