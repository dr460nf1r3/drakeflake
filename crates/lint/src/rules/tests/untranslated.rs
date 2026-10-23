use super::count;
use crate::source::Lang;

const RULE: &str = "no-untranslated-text";

#[test]
fn literal_labels_are_reported() {
    assert_eq!(
        count(Lang::Qml, RULE, "Label {\n    text: \"Hello\"\n}\n"),
        1
    );
    assert_eq!(
        count(
            Lang::Qml,
            RULE,
            "Action {\n    text: ok ? \"Yes\" : qsTr(\"No\")\n}\n"
        ),
        1
    );
    assert_eq!(
        count(Lang::Qml, RULE, "Label {\n    ToolTip.text: \"Copy\"\n}\n"),
        1
    );
}

#[test]
fn translated_and_non_text_values_pass() {
    let text = "Label {\n    text: qsTr(\"Hello %1\").arg(name)\n    title: \"•\"\n    icon.name: \"edit-copy\"\n    property color text: \"transparent\"\n    subtitle: Helpers.richText(\"x\")\n}\n";
    assert_eq!(count(Lang::Qml, RULE, text), 0);
}

#[test]
fn nested_objects_comparisons_markup_and_paths_pass() {
    let text = "Item {\n    header: Icon {\n        fallback: \"preferences-system\"\n    }\n    text: row.check === \"pending\" ? qsTr(\"Waiting\") : \"✖ \"\n    title: \"<b>\" + name + \"</b><br>\" + \"\\n\"\n    placeholderText: \"/etc/nixos\"\n}\n";
    assert_eq!(count(Lang::Qml, RULE, text), 0);
}
