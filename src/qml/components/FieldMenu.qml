pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls

// Spelling and substitutions need platform text views, which Qt does not expose.
Item {
    id: root

    required property var target
    readonly property alias selectAllAction: selectAllAction

    function canEdit(name: string): bool {
        return root.target !== null && typeof root.target[name] === "function";
    }

    MouseArea {
        parent: root.target
        anchors.fill: parent
        acceptedButtons: Qt.RightButton

        onClicked: menu.popup()
    }

    Controls.Menu {
        id: menu

        Controls.Action {
            text: qsTr("Undo")
            enabled: root.canEdit("undo") && root.target.canUndo === true

            onTriggered: root.target.undo()
        }

        Controls.Action {
            text: qsTr("Redo")
            enabled: root.canEdit("redo") && root.target.canRedo === true

            onTriggered: root.target.redo()
        }

        Controls.MenuSeparator {
        }

        Controls.Action {
            text: qsTr("Cut")
            enabled: root.canEdit("cut") && root.target.readOnly !== true && root.target.selectedText !== ""

            onTriggered: root.target.cut()
        }

        Controls.Action {
            text: qsTr("Copy")
            enabled: root.canEdit("copy") && root.target.selectedText !== ""

            onTriggered: root.target.copy()
        }

        Controls.Action {
            text: qsTr("Paste")
            enabled: root.canEdit("paste") && root.target.canPaste === true

            onTriggered: root.target.paste()
        }

        Controls.Action {
            id: selectAllAction

            text: qsTr("Select All")
            enabled: root.canEdit("selectAll")

            onTriggered: root.target.selectAll()
        }
    }
}
