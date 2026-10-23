use tree_sitter::Node;

use crate::finding::Finding;
use crate::source::{Lang, Source};

const RULE: &str = "qml-function-types";

pub fn check(source: &Source<'_>, findings: &mut Vec<Finding>) {
    if source.lang != Lang::Qml {
        return;
    }

    for function in source.nodes_of_kind("function_declaration") {
        let member = function
            .parent()
            .is_some_and(|parent| parent.kind() == "ui_object_initializer");

        if !member {
            continue;
        }

        let name = function
            .child_by_field_name("name")
            .map_or("", |name| source.node_text(name));

        let untyped = untyped_parameters(source, function);
        let no_result = function.child_by_field_name("return_type").is_none();

        if untyped.is_empty() && !no_result {
            continue;
        }

        let mut missing = untyped
            .iter()
            .map(|parameter| format!("`{parameter}`"))
            .collect::<Vec<_>>();

        if no_result {
            missing.push("the result".to_owned());
        }

        findings.push(
            Finding::new(
                function.start_position(),
                RULE,
                format!("Function `{name}` has no type for {}.", missing.join(", ")),
                "The QML compiler skips untyped functions, and callers cannot see what goes in and comes out.",
            )
            .fix("Annotate each parameter (`name: string`) and the result (`): void`, `): var`)."),
        );
    }
}

fn untyped_parameters<'t>(source: &Source<'t>, function: Node<'t>) -> Vec<&'t str> {
    let Some(parameters) = function.child_by_field_name("parameters") else {
        return Vec::new();
    };

    let mut cursor = parameters.walk();
    parameters
        .named_children(&mut cursor)
        .filter(|parameter| parameter.child_by_field_name("type").is_none())
        .map(|parameter| {
            parameter
                .child_by_field_name("name")
                .or_else(|| parameter.child_by_field_name("pattern"))
                .map_or("", |name| source.node_text(name))
        })
        .collect()
}
