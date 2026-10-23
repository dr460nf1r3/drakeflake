pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"

GridLayout {
    id: editor

    required property string optionName
    required property bool editable
    required property var info
    required property string editText
    required property string inputMode
    required property string priority
    required property var priorityInfo
    required property string shownPriority
    required property bool hostScopeAllowed
    required property string scopeHost
    required property string currentHost
    required property string hostState
    readonly property bool expressionMode: editor.inputMode === "expression"

    signal edited(string text)
    signal settingsEdited(string mode, string priority, string host)

    columns: 2
    columnSpacing: Kirigami.Units.largeSpacing
    rowSpacing: Kirigami.Units.smallSpacing

    FieldLabel {
        text: qsTr("Enter as:")
    }

    Controls.ComboBox {
        id: modeBox

        objectName: "inputModeBox"
        Layout.minimumWidth: Kirigami.Units.gridUnit * 12
        textRole: "text"
        valueRole: "value"
        model: [
            {
                value: "typed",
                text: qsTr("Value")
            },
            {
                value: "expression",
                text: qsTr("Nix expression")
            }
        ]
        currentIndex: editor.expressionMode ? 1 : 0

        onActivated: editor.settingsEdited(modeBox.currentValue, editor.priority, editor.scopeHost)
    }

    FieldLabel {
        text: qsTr("Value:")
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: 0

        // One editor instance per option: switching options must never keep
        // the previous option's typed text or validation state.
        Repeater {
            model: editor.editable && !editor.expressionMode ? [editor.optionName] : []

            delegate: ValueEditor {
                required property string modelData

                Layout.fillWidth: true
                info: editor.info
                text: editor.editText

                onEdited: text => editor.edited(text)
            }
        }

        MultiLineField {
            id: expressionField

            objectName: "expressionField"
            visible: editor.expressionMode
            Layout.fillWidth: true
            font.family: "monospace"
            placeholderText: qsTr("Nix expression with config, lib and pkgs, e.g. config.networking.hostName + \"-vm\"")

            // Assigned (not bound) while not editing, so typing keeps the cursor.
            Binding on text {
                when: !expressionField.editing
                value: editor.editText
                restoreMode: Binding.RestoreNone
            }

            onTextEdited: text => {
                if (text !== editor.editText)
                    editor.edited(text);
            }
        }
    }

    FieldLabel {
        text: qsTr("Priority:")
    }

    PriorityPicker {
        Layout.fillWidth: true
        choices: editor.priorityInfo.choices
        implicitChoice: editor.priorityInfo.implicit
        shownPriority: editor.shownPriority

        onPicked: priority => editor.settingsEdited(editor.inputMode, priority, editor.scopeHost)
    }

    FieldLabel {
        visible: editor.hostScopeAllowed
        text: qsTr("Applies to:")
    }

    HostScopePicker {
        Layout.fillWidth: true
        visible: editor.hostScopeAllowed
        scopeHost: editor.scopeHost
        currentHost: editor.currentHost
        hostState: editor.hostState

        onPicked: host => editor.settingsEdited(editor.inputMode, editor.priority, host)
    }
}
