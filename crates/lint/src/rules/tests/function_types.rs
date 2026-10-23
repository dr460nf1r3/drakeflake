use super::count;
use crate::source::Lang;

const RULE: &str = "qml-function-types";

#[test]
fn typed_function_passes() {
    let text = "Item {\n    function shot(name: string, wait: int): void {\n        go(name, wait);\n    }\n}\n";
    assert_eq!(count(Lang::Qml, RULE, text), 0);
}

#[test]
fn untyped_parameter_is_reported() {
    let text = "Item {\n    function shot(name: string, wait): void {\n        go(name, wait);\n    }\n}\n";
    assert_eq!(count(Lang::Qml, RULE, text), 1);
}

#[test]
fn missing_result_type_is_reported() {
    let text = "Connections {\n    function onChanged() {\n        refresh();\n    }\n}\n";
    assert_eq!(count(Lang::Qml, RULE, text), 1);
}

#[test]
fn problem_names_what_is_missing() {
    let findings = crate::rules::lint_text(
        Lang::Qml,
        "Item {\n    function shot(name, wait: int) {\n        go(name, wait);\n    }\n}\n",
    );

    let hit = findings.iter().find(|finding| finding.rule == RULE);
    assert_eq!(
        hit.map(|finding| finding.problem.as_str()),
        Some("Function `shot` has no type for `name`, the result.")
    );
}

#[test]
fn nested_functions_pass() {
    let text = "Item {\n    function outer(): void {\n        function inner(x) {\n            return x;\n        }\n        inner(1);\n    }\n}\n";
    assert_eq!(count(Lang::Qml, RULE, text), 0);
}

#[test]
fn javascript_files_pass() {
    let text = "function shot(name) {\n    go(name);\n}\n";
    assert_eq!(count(Lang::Js, RULE, text), 0);
}
