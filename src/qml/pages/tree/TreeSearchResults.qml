pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"

ListView {
    id: results

    required property var app
    required property var options
    required property string scope
    required property string homePrefix

    function showName(name: string): string {
        if (results.scope === "home" && name.startsWith(results.homePrefix))
            return name.slice(results.homePrefix.length);
        return name;
    }

    currentIndex: -1

    delegate: Controls.ItemDelegate {
        id: hit

        required property var modelData
        required property int index
        readonly property string name: hit.modelData.name
        readonly property string value: hit.modelData.value ?? ""

        width: ListView.view.width
        highlighted: false

        background: SelectionBackground {
            selected: hit.ListView.isCurrentItem
            hovered: hit.hovered
        }
        contentItem: ColumnLayout {
            spacing: 0

            Controls.Label {
                Layout.fillWidth: true
                textFormat: Text.PlainText
                text: results.showName(hit.name)
                font.family: "monospace"
                font.bold: true
                wrapMode: Text.WrapAnywhere
            }

            Controls.Label {
                Layout.fillWidth: true
                visible: hit.value.length > 0
                textFormat: Text.PlainText
                text: "= " + hit.value
                font.family: "monospace"
                color: Kirigami.Theme.linkColor
                elide: Text.ElideRight
            }
        }

        onClicked: {
            results.currentIndex = hit.index;
            results.app.showOption(hit.name, results.options);
        }
    }

    Kirigami.PlaceholderMessage {
        anchors.centerIn: parent
        width: parent.width - Kirigami.Units.gridUnit * 4
        visible: results.count === 0
        icon.name: "search"
        text: qsTr("No matching options")
        explanation: qsTr("Try fewer or different words.")
    }
}
