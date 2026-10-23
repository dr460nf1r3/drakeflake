use std::sync::LazyLock;

use regex::Regex;
use tree_sitter::Node;

use super::{Lang, parse};
use crate::pattern::re;

static PROSE_MARKER: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^(TODO|FIXME|HACK|NOTE|XXX|REVIEW)\b|^https?://"));

const BODIES: &[&str] = &["block", "statement_block", "ui_object_initializer"];

const TRIVIAL: &[&str] = &[
    "identifier",
    "field_expression",
    "scoped_identifier",
    "member_expression",
    "nested_identifier",
    "string",
    "string_literal",
    "raw_string_literal",
    "char_literal",
    "integer_literal",
    "float_literal",
    "number",
    "true",
    "false",
    "boolean_literal",
    "self",
    "this",
    "template_string",
    "labeled_statement",
    "empty_statement",
    "line_comment",
    "block_comment",
    "comment",
];

pub fn looks_like_code(lang: Lang, text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || PROSE_MARKER.is_match(trimmed) {
        return false;
    }

    match lang {
        Lang::Rust => {
            parses_as_code(Lang::Rust, trimmed, "", "")
                || parses_as_code(Lang::Rust, trimmed, "fn wrapper() {\n", "\n}")
        }
        Lang::Qml | Lang::Js => {
            parses_as_code(Lang::Js, trimmed, "function wrapper() {\n", "\n}")
                || parses_as_code(Lang::Qml, trimmed, "Item {\n", "\n}")
        }
    }
}

fn parses_as_code(lang: Lang, text: &str, before: &str, after: &str) -> bool {
    let wrapped = format!("{before}{text}{after}");
    let Some(tree) = parse(lang, &wrapped) else {
        return false;
    };

    let root = tree.root_node();
    if root.has_error() {
        return false;
    }

    let body = if before.is_empty() {
        Some(root)
    } else {
        find_body(root)
    };

    body.is_some_and(|body| {
        let mut cursor = body.walk();
        let mut statements = body.named_children(&mut cursor).peekable();
        statements.peek().is_some() && !statements.all(is_trivial)
    })
}

fn find_body(root: Node<'_>) -> Option<Node<'_>> {
    let mut stack = vec![root];

    while let Some(node) = stack.pop() {
        if BODIES.contains(&node.kind()) {
            return Some(node);
        }

        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }

    None
}

fn is_trivial(node: Node<'_>) -> bool {
    if TRIVIAL.contains(&node.kind()) {
        return true;
    }

    let mut cursor = node.walk();
    let value = node.named_children(&mut cursor).last();
    match node.kind() {
        "expression_statement" => value.is_some_and(is_trivial),
        "sequence_expression" => {
            let mut parts = node.walk();
            node.named_children(&mut parts).all(is_trivial)
        }
        "ui_binding" => value.is_some_and(|value| {
            let mut value_cursor = value.walk();
            let inner: Vec<Node<'_>> = value.named_children(&mut value_cursor).collect();
            matches!(inner.as_slice(), [word] if word.kind() == "identifier")
        }),
        _ => false,
    }
}
