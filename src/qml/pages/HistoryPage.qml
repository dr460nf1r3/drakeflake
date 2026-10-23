pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../main/helpers.js" as Helpers

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var system: page.app.systemModel
    readonly property var generationsModel: page.app.generationsModel
    readonly property var entries: page.generationsModel.history

    function reload(): void {
        page.generationsModel.load_history();
    }

    title: qsTr("History")

    actions: [
        Kirigami.Action {
            text: qsTr("Refresh")
            icon.name: "view-refresh"
            displayHint: Kirigami.DisplayHint.IconOnly
            enabled: !page.generationsModel.history_loading

            onTriggered: page.reload()
        }
    ]

    Component.onCompleted: page.reload()

    Connections {
        function onState_changed(): void {
            if (!page.system.running)
                page.reload();
        }

        target: page.system
    }

    ListView {
        id: list

        model: page.entries

        header: Controls.Label {
            width: ListView.view ? ListView.view.width : 0
            padding: Kirigami.Units.largeSpacing
            wrapMode: Text.Wrap
            opacity: 0.75
            text: qsTr("Each save keeps the settings it replaced. Restoring writes them back (and keeps the current ones here). Rebuild afterwards to use them.")
        }
        delegate: Controls.Control {
            id: row

            required property var modelData

            width: ListView.view.width
            padding: Kirigami.Units.largeSpacing

            background: Item {
                Kirigami.Separator {
                    anchors {
                        left: parent.left
                        right: parent.right
                        bottom: parent.bottom
                    }
                }
            }
            contentItem: RowLayout {
                spacing: Kirigami.Units.largeSpacing

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: Kirigami.Units.smallSpacing

                    Controls.Label {
                        font.bold: true
                        textFormat: Text.PlainText
                        text: row.modelData.saved ? qsTr("Replaced %1").arg(Helpers.formatDate(row.modelData.saved)) : qsTr("Earlier settings")
                    }

                    Controls.Label {
                        opacity: 0.75
                        textFormat: Text.PlainText
                        text: qsTr("%1 settings · %2 packages").arg(row.modelData.options).arg(row.modelData.packages)
                    }

                    Controls.Label {
                        Layout.fillWidth: true
                        visible: row.modelData.changes.length === 0
                        opacity: 0.6
                        text: qsTr("Same as now")
                    }

                    Repeater {
                        model: row.modelData.changes.slice(0, 8)

                        delegate: Controls.Label {
                            id: change

                            required property string modelData

                            Layout.fillWidth: true
                            font.family: "monospace"
                            font.pointSize: Kirigami.Theme.smallFont.pointSize
                            wrapMode: Text.WrapAnywhere
                            text: change.modelData
                        }
                    }

                    Controls.Label {
                        visible: row.modelData.changes.length > 8
                        opacity: 0.6
                        font: Kirigami.Theme.smallFont
                        textFormat: Text.PlainText
                        text: qsTr("… and %1 more").arg(row.modelData.changes.length - 8)
                    }
                }

                Controls.Button {
                    text: qsTr("Restore")
                    icon.name: "document-revert"
                    icon.width: Kirigami.Units.iconSizes.sizeForLabels
                    icon.height: Kirigami.Units.iconSizes.sizeForLabels
                    enabled: !page.system.running && row.modelData.changes.length > 0

                    onClicked: {
                        page.system.restore_history(row.modelData.file);
                        page.app.showSection("log");
                    }
                }
            }
        }

        Kirigami.PlaceholderMessage {
            anchors.centerIn: parent
            width: parent.width - Kirigami.Units.gridUnit * 4
            visible: list.count === 0
            icon.name: "view-history"
            text: page.generationsModel.history_loading ? qsTr("Reading earlier settings…") : qsTr("No earlier settings yet")
            explanation: page.generationsModel.history_loading ? "" : qsTr("Every save keeps the previous settings here.")
        }
    }
}
