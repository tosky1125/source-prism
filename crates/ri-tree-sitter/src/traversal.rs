#![allow(
    clippy::redundant_pub_crate,
    reason = "Parent module consumes this private-module traversal entry point."
)]

use ri_core::{Language, SymbolKind};
use ri_parser::SourceFile;
use ri_symbols::{SymbolRange, SymbolRecord, SymbolSpec};
use tree_sitter::Node;

use crate::names::{field_name, go_receiver_type, rust_impl_type};

pub(crate) fn extract_tree_symbols(file: &SourceFile<'_>, root: Node<'_>) -> Vec<SymbolRecord> {
    let mut symbols = Vec::new();
    walk(file, root, &mut Vec::new(), &mut symbols);
    symbols
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ContainerKind {
    Impl,
    Module,
    Class,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Container {
    name: String,
    kind: ContainerKind,
}

fn walk(
    file: &SourceFile<'_>,
    node: Node<'_>,
    parents: &mut Vec<Container>,
    symbols: &mut Vec<SymbolRecord>,
) {
    let symbol = symbol_for_node(file, node, parents);
    let parent_name = symbol
        .as_ref()
        .map(|record| Container {
            name: record.name.clone(),
            kind: container_kind_for_symbol(record.kind),
        })
        .or_else(|| container_name(file, node));
    if let Some(container) = &parent_name {
        parents.push(container.clone());
    }
    if let Some(record) = symbol {
        symbols.push(record);
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        walk(file, child, parents, symbols);
    }

    if parent_name.is_some() {
        let _ = parents.pop();
    }
}

fn symbol_for_node(
    file: &SourceFile<'_>,
    node: Node<'_>,
    parents: &[Container],
) -> Option<SymbolRecord> {
    let kind = symbol_kind(file.language, node.kind(), parents)?;
    let name = symbol_name(file, node)?;
    let fqn = qualified_name(parents, &name);
    Some(SymbolRecord::new(
        &file.repo,
        &file.commit,
        file.path.clone(),
        file.content_hash,
        SymbolSpec::new(file.language, kind, name, fqn, range_for(node)),
    ))
}

fn symbol_name(file: &SourceFile<'_>, node: Node<'_>) -> Option<String> {
    match (file.language, node.kind()) {
        (Language::Rust, "impl_item") => rust_impl_type(file.source, node),
        (Language::Go, "method_declaration") => {
            let receiver = go_receiver_type(file.source, node)?;
            let name = field_name(file.source, node, "name")?;
            Some(format!("{receiver}.{name}"))
        }
        _ => field_name(file.source, node, "name"),
    }
}

fn symbol_kind(language: Language, node_kind: &str, parents: &[Container]) -> Option<SymbolKind> {
    match (language, node_kind) {
        (Language::Rust, "mod_item") | (Language::Go, "package_clause") => Some(SymbolKind::Module),
        (Language::Rust, "struct_item")
        | (Language::Python, "class_definition")
        | (Language::TypeScript | Language::JavaScript, "class_declaration") => {
            Some(SymbolKind::Class)
        }
        (Language::Rust, "enum_item")
        | (Language::TypeScript | Language::JavaScript, "enum_declaration") => {
            Some(SymbolKind::Enum)
        }
        (Language::TypeScript | Language::JavaScript, "interface_declaration") => {
            Some(SymbolKind::Interface)
        }
        (Language::Rust, "function_item")
            if parents
                .last()
                .is_some_and(|parent| parent.kind == ContainerKind::Impl) =>
        {
            Some(SymbolKind::Method)
        }
        (Language::Python, "function_definition")
            if parents
                .last()
                .is_some_and(|parent| parent.kind == ContainerKind::Class) =>
        {
            Some(SymbolKind::Method)
        }
        (_, "function_item" | "function_declaration" | "function_definition") => {
            Some(SymbolKind::Function)
        }
        (_, "method_definition" | "method_declaration") => Some(SymbolKind::Method),
        _ => None,
    }
}

fn container_name(file: &SourceFile<'_>, node: Node<'_>) -> Option<Container> {
    match (file.language, node.kind()) {
        (Language::Rust, "impl_item") => rust_impl_type(file.source, node).map(|name| Container {
            name,
            kind: ContainerKind::Impl,
        }),
        _ => None,
    }
}

const fn container_kind_for_symbol(kind: SymbolKind) -> ContainerKind {
    match kind {
        SymbolKind::Class
        | SymbolKind::Interface
        | SymbolKind::Enum
        | SymbolKind::Constructor
        | SymbolKind::Field
        | SymbolKind::RouteHandler
        | SymbolKind::TestCase => ContainerKind::Class,
        _ => ContainerKind::Module,
    }
}

fn qualified_name(parents: &[Container], name: &str) -> String {
    if parents.is_empty() {
        name.to_owned()
    } else {
        let prefix = parents
            .iter()
            .map(|parent| parent.name.as_str())
            .collect::<Vec<_>>()
            .join("::");
        format!("{prefix}::{name}")
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
