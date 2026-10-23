use tree_sitter::Node;

use crate::finding::Finding;
use crate::rules::excerpt;
use crate::source::{Kind, Lang, Source, Unit};

const RULE: &str = "blank-before-comment";

pub fn check(source: &Source<'_>, findings: &mut Vec<Finding>) {
    for unit in source.units() {
        let first = unit.first();
        if !matches!(unit.kind(), Kind::Line | Kind::Block) || !first.alone {
            continue;
        }

        let between_steps = first
            .node
            .parent()
            .is_some_and(|parent| holds_steps(source.lang, parent.kind()));

        let after_attribute = first
            .node
            .prev_named_sibling()
            .is_some_and(|node| node.kind() == "attribute_item");

        if !between_steps || after_attribute || inside_qml_section(source, &unit) {
            continue;
        }

        let start = unit.start();
        let Some(above) = start.row.checked_sub(1) else {
            continue;
        };

        let line_above = source.lines.get(above).copied().unwrap_or_default();
        if may_touch(source.lang, line_above.trim()) {
            continue;
        }

        findings.push(
            Finding::new(
                start,
                RULE,
                format!(
                    "Comment \"{}\" touches the line above.",
                    excerpt(&unit.text())
                ),
                "A comment glued to the code above reads as part of it, not as the heading of the step below.",
            )
            .fix("Add a blank line above the comment."),
        );
    }
}

fn holds_steps(lang: Lang, kind: &str) -> bool {
    match lang {
        Lang::Rust => matches!(kind, "block" | "source_file" | "declaration_list"),
        Lang::Qml | Lang::Js => matches!(
            kind,
            "program"
                | "statement_block"
                | "class_body"
                | "switch_case"
                | "switch_default"
                | "ui_object_initializer"
        ),
    }
}

fn inside_qml_section(source: &Source<'_>, unit: &Unit<'_, '_>) -> bool {
    let in_object = unit
        .first()
        .node
        .parent()
        .is_some_and(|parent| parent.kind() == "ui_object_initializer");

    if source.lang != Lang::Qml || !in_object {
        return false;
    }

    let above = neighbour(source, unit.first().node, Node::prev_named_sibling);
    let below = neighbour(source, unit.last().node, Node::next_named_sibling);
    above.is_some() && above.map(section) == below.map(section)
}

fn neighbour<'t>(
    source: &Source<'_>,
    node: Node<'t>,
    step: impl Fn(&Node<'t>) -> Option<Node<'t>>,
) -> Option<Node<'t>> {
    let mut next = step(&node);
    while let Some(found) = next.filter(|found| source.lang.is_comment(found.kind())) {
        next = step(&found);
    }

    next
}

fn section(member: Node<'_>) -> &'static str {
    let member = if member.kind() == "ui_annotated_object_member" {
        member.child_by_field_name("definition").unwrap_or(member)
    } else {
        member
    };

    match member.kind() {
        "ui_property" | "ui_required" => "declarations",
        "ui_signal" => "signals",
        "function_declaration" | "generator_function_declaration" => "functions",
        "ui_binding" | "ui_object_definition_binding" => "bindings",
        "ui_object_definition" | "ui_inline_component" => "children",
        _ => "other",
    }
}

fn may_touch(lang: Lang, above: &str) -> bool {
    if above.is_empty() || above.ends_with(['{', '(', '[']) {
        return true;
    }

    if above.starts_with("//") || above.ends_with("*/") {
        return true;
    }

    lang != Lang::Rust && is_case_label(above)
}

fn is_case_label(line: &str) -> bool {
    line.ends_with(':') && (line.starts_with("case ") || line == "default:")
}
