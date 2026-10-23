import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

import "../components"

Kirigami.Page {
    id: page

    required property var app
    readonly property bool isPreview: true
    property string text: ""
    property string pendingMode: ""

    function refresh(): void {
        page.text = page.app.changeModel.preview(page.app.changeModel.plan_json());
    }

    title: qsTr("Preview")
    padding: 0

    actions: [
        Kirigami.Action {
            text: qsTr("Apply…")
            icon.name: "dialog-ok-apply"
            visible: page.pendingMode !== ""
            tooltip: qsTr("Save the file and rebuild, asking before it activates")

            onTriggered: page.app.reviewApply(page.pendingMode)
        },
        Kirigami.Action {
            text: qsTr("Copy")
            icon.name: "edit-copy"

            onTriggered: {
                edit.selectAll();
                edit.copy();
                edit.deselect();
            }
        },
        Kirigami.Action {
            text: qsTr("Close")
            icon.name: "dialog-close"

            onTriggered: page.app.pageStack.removePage(page)
        }
    ]

    Component.onCompleted: page.refresh()

    Connections {

        // Deferred: never call back into a model from inside its own signal.
        function onChanged(): void {
            Qt.callLater(page.refresh);
        }

        target: page.app.changeModel
    }

    Rectangle {
        id: background

        anchors.fill: parent
        Kirigami.Theme.colorSet: Kirigami.Theme.View
        Kirigami.Theme.inherit: false
        color: highlighter.hasColors ? highlighter.backgroundColor : Kirigami.Theme.backgroundColor

        Flickable {
            id: flick

            anchors.fill: parent
            clip: true
            contentWidth: Math.max(width, edit.implicitWidth)
            contentHeight: Math.max(height, edit.implicitHeight)
            boundsBehavior: Flickable.StopAtBounds

            Controls.ScrollBar.vertical: Controls.ScrollBar {
            }
            Controls.ScrollBar.horizontal: Controls.ScrollBar {
            }

            TextEdit {
                id: edit

                width: flick.contentWidth
                height: flick.contentHeight
                padding: Kirigami.Units.largeSpacing
                readOnly: true
                selectByMouse: true
                textFormat: TextEdit.PlainText
                wrapMode: TextEdit.NoWrap
                font.family: "monospace"
                color: highlighter.hasColors ? highlighter.textColor : Kirigami.Theme.textColor
                selectionColor: Kirigami.Theme.highlightColor
                selectedTextColor: Kirigami.Theme.highlightedTextColor
                text: page.text

                // "### path" headers are Nix comments, so the whole preview highlights.
                NixHighlighter {
                    id: highlighter

                    textEdit: edit
                    themeName: page.app.highlightTheme
                    dark: Kirigami.ColorUtils.brightnessForColor(background.Kirigami.Theme.backgroundColor) === Kirigami.ColorUtils.Dark
                }
            }
        }
    }
}
