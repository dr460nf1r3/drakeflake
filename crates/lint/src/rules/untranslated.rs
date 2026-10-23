use std::sync::LazyLock;

use regex::Regex;
use tree_sitter::Node;

use super::excerpt;
use crate::finding::Finding;
use crate::pattern::re;
use crate::source::{Lang, Source};

const RULE: &str = "no-untranslated-text";

static WORD: LazyLock<Regex> = LazyLock::new(|| re(r"\p{L}{2,}"));
static TAG: LazyLock<Regex> = LazyLock::new(|| re("<[^>]*>"));

const LABELS: &[&str] = &[
    "text",
    "title",
    "subtitle",
    "description",
    "explanation",
    "label",
    "placeholderText",
    "tooltip",
    "header",
];

pub fn check(source: &Source<'_>, findings: &mut Vec<Finding>) {
    if source.lang != Lang::Qml {
        return;
    }

    for binding in source.nodes_of_kind("ui_binding") {
        let Some(name) = binding.named_child(0).map(|name| source.node_text(name)) else {
            continue;
        };

        let property = name.rsplit('.').next().unwrap_or(name);
        let Some(value) = binding.named_child(1) else {
            continue;
        };

        if !LABELS.contains(&property) {
            continue;
        }

        for string in strings_shown(source, value) {
            let text = source.node_text(string);
            if !is_prose(source, string) {
                continue;
            }

            findings.push(
                Finding::new(
                    string.start_position(),
                    RULE,
                    format!("Untranslated `{name}: {}`.", excerpt(text)),
                    "Literal labels are shown in one language only.",
                )
                .fix("Wrap the text in `qsTr(\"…\")` (with `.arg()` for values), so it lands in the translation files."),
            );
        }
    }
}

fn strings_shown<'t>(source: &Source<'t>, value: Node<'t>) -> Vec<Node<'t>> {
    let mut found = Vec::new();
    let mut stack = vec![value];

    while let Some(node) = stack.pop() {
        match node.kind() {
            "arguments"
            | "statement_block"
            | "function_expression"
            | "arrow_function"
            | "ui_object_definition"
            | "ui_object_initializer" => {}
            "binary_expression" if is_comparison(source, node) => {}
            "string" => found.push(node),
            _ => {
                let mut cursor = node.walk();
                let children: Vec<Node<'_>> = node.named_children(&mut cursor).collect();
                stack.extend(children);
            }
        }
    }

    found
}

fn is_comparison(source: &Source<'_>, node: Node<'_>) -> bool {
    node.child_by_field_name("operator")
        .is_some_and(|operator| matches!(source.node_text(operator), "===" | "!==" | "==" | "!="))
}

fn is_prose(source: &Source<'_>, string: Node<'_>) -> bool {
    let mut cursor = string.walk();
    let fragments: String = string
        .named_children(&mut cursor)
        .filter(|part| part.kind() == "string_fragment")
        .map(|part| source.node_text(part))
        .collect();

    let path = fragments.starts_with('/') || fragments.starts_with('~');
    !path && WORD.is_match(&TAG.replace_all(&fragments, ""))
}
