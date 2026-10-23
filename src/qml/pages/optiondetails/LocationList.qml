pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../main/helpers.js" as Helpers

ColumnLayout {
    id: list

    required property var app
    required property var locations

    signal opened(string file, int line)

    spacing: 0

    Controls.Label {
        text: qsTr("Set in:")
        opacity: 0.75
        font: Kirigami.Theme.smallFont
    }

    Repeater {
        model: list.locations

        delegate: Controls.Label {
            id: location

            required property var modelData

            Layout.fillWidth: true
            textFormat: Text.StyledText
            wrapMode: Text.WrapAnywhere
            font.family: "monospace"
            text: Helpers.link("open", qsTr("%1:%2").arg(location.modelData.display).arg(location.modelData.line)) + (location.modelData.local ? "" : " " + qsTr("(flake input, read-only)"))

            onLinkActivated: list.opened(location.modelData.file, location.modelData.line)

            HoverHandler {
                cursorShape: Qt.PointingHandCursor
            }
        }
    }
}
