#![allow(
    clippy::redundant_pub_crate,
    reason = "Sibling traversal module consumes these private-module helpers."
)]

use tree_sitter::Node;

pub(crate) fn node_text(source: &str, node: Node<'_>) -> Option<String> {
    node.utf8_text(source.as_bytes()).ok().map(str::to_owned)
}

pub(crate) fn field_name(source: &str, node: Node<'_>, field: &str) -> Option<String> {
    node.child_by_field_name(field)
        .and_then(|child| node_text(source, child))
}

pub(crate) fn rust_impl_type(source: &str, node: Node<'_>) -> Option<String> {
    node.child_by_field_name("type")
        .and_then(|child| node_text(source, child))
}

pub(crate) fn go_receiver_type(source: &str, node: Node<'_>) -> Option<String> {
    let receiver = node.child_by_field_name("receiver")?;
    let mut cursor = receiver.walk();
    for child in receiver.children(&mut cursor) {
        if child.kind() == "parameter_declaration" {
            return child
                .child_by_field_name("type")
                .and_then(|type_node| node_text(source, type_node))
                .map(|raw| raw.trim_start_matches('*').to_owned());
        }
    }
    None
}
