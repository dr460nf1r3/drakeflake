import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../main/kinds.js" as Kinds

ColumnLayout {
    id: editor

    property var info: ({})
    property string text: ""
    readonly property string kind: editor.info.kind || Kinds.editorRaw
    readonly property bool nullable: editor.info.nullable === true
    readonly property bool isNull: editor.nullable && editor.text === "null"

    // SpinBox is limited to 32-bit. Wider ranges use a text field.
    readonly property bool spinnable: editor.kind === Kinds.editorInt && editor.info.min !== null && editor.info.min !== undefined && editor.info.max !== null && editor.info.max !== undefined && editor.info.min >= -2147483647 && editor.info.max <= 2147483647

    signal edited(string text)

    spacing: Kirigami.Units.smallSpacing

    Controls.CheckBox {
        objectName: "nullCheck"
        visible: editor.nullable
        text: qsTr("Unset (null)")
        checked: editor.isNull

        onToggled: editor.edited(checked ? "null" : (editor.kind === Kinds.editorBool ? "false" : ""))
    }

    Controls.Switch {
        objectName: "boolSwitch"
        visible: editor.kind === Kinds.editorBool
        enabled: !editor.isNull
        text: checked ? qsTr("Enabled") : qsTr("Disabled")
        checked: editor.text === "true"

        onToggled: editor.edited(checked ? "true" : "false")
    }

    Controls.SpinBox {
        objectName: "intSpin"
        visible: editor.spinnable
        enabled: !editor.isNull
        editable: true
        from: editor.spinnable ? editor.info.min : 0
        to: editor.spinnable ? editor.info.max : 0
        value: {
            const parsed = parseInt(editor.text);
            return isNaN(parsed) ? from : parsed;
        }
        Layout.minimumWidth: Kirigami.Units.gridUnit * 8

        onValueModified: editor.edited(String(value))
    }

    Controls.ComboBox {
        objectName: "enumCombo"
        visible: editor.kind === Kinds.editorEnum
        enabled: !editor.isNull
        model: editor.info.choices || []
        currentIndex: (editor.info.choices || []).indexOf(editor.text)
        displayText: currentIndex < 0 ? qsTr("Choose…") : currentText
        Layout.minimumWidth: Kirigami.Units.gridUnit * 10

        onActivated: editor.edited(currentText)
    }

    Controls.TextField {
        id: valueField

        objectName: "valueField"
        visible: (editor.kind === Kinds.editorInt && !editor.spinnable) || [Kinds.editorFloat, Kinds.editorString, Kinds.editorPath, Kinds.editorPackage].indexOf(editor.kind) >= 0
        enabled: !editor.isNull
        Layout.fillWidth: true
        text: editor.isNull ? "" : editor.text
        font.family: editor.kind === Kinds.editorPath || editor.kind === Kinds.editorPackage ? "monospace" : Kirigami.Theme.defaultFont.family
        placeholderText: {
            switch (editor.kind) {
            case Kinds.editorInt:
                return qsTr("Whole number");
            case Kinds.editorFloat:
                return qsTr("Number");
            case Kinds.editorPath:
                return "/absolute/path";
            case Kinds.editorPackage:
                return qsTr("Package attribute, e.g. firefox or kdePackages.kate");
            default:
                return qsTr("Text");
            }
        }
        inputMethodHints: editor.kind === Kinds.editorInt || editor.kind === Kinds.editorFloat ? Qt.ImhFormattedNumbersOnly : Qt.ImhNone

        onTextEdited: editor.edited(text)

        FieldMenu {
            target: valueField
        }
    }

    MultiLineField {
        id: area

        objectName: "area"
        visible: [Kinds.editorText, Kinds.editorList, Kinds.editorRaw].indexOf(editor.kind) >= 0
        enabled: !editor.isNull
        Layout.fillWidth: true
        font.family: editor.kind === Kinds.editorRaw || editor.kind === Kinds.editorList ? "monospace" : Kirigami.Theme.defaultFont.family
        placeholderText: {
            switch (editor.kind) {
            case Kinds.editorList:
                return qsTr("One item per line");
            case Kinds.editorRaw:
                return qsTr("Nix expression, e.g. { foo = \"bar\"; }");
            default:
                return qsTr("Text");
            }
        }

        // Assigned (not bound) while not editing, so typing keeps the cursor.
        Binding on text {
            when: !area.editing
            value: editor.isNull ? "" : editor.text
            // Keep the text when editing starts (the default restores the old value).
            restoreMode: Binding.RestoreNone
        }

        onTextEdited: text => {
            if (text !== editor.text)
                editor.edited(text);
        }
    }
}
