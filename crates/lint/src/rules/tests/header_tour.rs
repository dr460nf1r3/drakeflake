use super::count;
use crate::source::Lang;

const RULE: &str = "no-header-tour";

#[test]
fn multiline_leading_qml_comments_are_reported() {
    let text = "import QtQuick\n\n// Every option as an expandable tree, read-only.\n// Folders start open and collapse in place.\nimport org.kde.kirigami as Kirigami\n";
    assert_eq!(count(Lang::Qml, RULE, text), 1);
}

#[test]
fn reasons_and_single_lines_pass() {
    let reason = "import QtQuick\n\n// Clipped because ComboBox popups paint outside itself.\n// The second line keeps the reason.\nimport QtQuick.Controls\n";
    assert_eq!(count(Lang::Qml, RULE, reason), 0);

    let single = "import QtQuick\n\n// Small rounded label.\nimport QtQuick.Controls\n";
    assert_eq!(count(Lang::Qml, RULE, single), 0);
}
