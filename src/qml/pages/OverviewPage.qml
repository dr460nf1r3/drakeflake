pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../components"
import "overview"

Kirigami.Page {
    id: page

    required property var app
    readonly property var system: page.app.systemModel
    readonly property var running: page.app.systemModel.target_ready && page.app.systemModel.capabilities.generations ? page.app.generationsModel.generations.find(generation => generation.running) || null : null
    readonly property alias search: overviewSearch

    function focusSearch(): void {
        overviewSearch.focusSearch();
    }

    title: qsTr("Overview")

    Component.onCompleted: {
        if (page.app.inputsModel.inputs.length === 0)
            page.app.inputsModel.load(false);
        page.app.generationsModel.load_generations();
        page.app.generationsModel.load_history();
        page.app.functionModel.load();
    }

    ColumnLayout {
        anchors.fill: parent
        spacing: Kirigami.Units.gridUnit

        ColumnLayout {
            id: content

            Layout.alignment: Qt.AlignHCenter | Qt.AlignTop
            Layout.fillWidth: true
            Layout.maximumWidth: Kirigami.Units.gridUnit * 40
            Layout.topMargin: Math.max(0, Math.min(Kirigami.Units.gridUnit * 4, page.availableHeight - content.implicitHeight - Kirigami.Units.gridUnit))
            spacing: Kirigami.Units.gridUnit

            ColumnLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                Logo {
                    Layout.alignment: Qt.AlignHCenter
                    Layout.preferredWidth: Kirigami.Units.iconSizes.huge
                    Layout.preferredHeight: Kirigami.Units.iconSizes.huge
                }

                Kirigami.Heading {
                    Layout.fillWidth: true
                    level: 1
                    text: page.system.host.length > 0 ? page.system.host : qsTr("No system chosen")
                    textFormat: Text.PlainText
                    horizontalAlignment: Text.AlignHCenter
                    elide: Text.ElideRight
                }

                Controls.Label {
                    Layout.fillWidth: true
                    visible: page.running !== null
                    text: page.running ? [qsTr("Generation %1").arg(page.running.id), page.running.version].filter(part => part && part.length > 0).join(" · ") : ""
                    textFormat: Text.PlainText
                    horizontalAlignment: Text.AlignHCenter
                    elide: Text.ElideRight
                    opacity: 0.75
                }

                Controls.Label {
                    Layout.fillWidth: true
                    visible: page.system.flake.length > 0
                    text: page.system.flake
                    textFormat: Text.PlainText
                    horizontalAlignment: Text.AlignHCenter
                    elide: Text.ElideMiddle
                    font.family: "monospace"
                    opacity: 0.75
                }
            }

            OverviewSearch {
                id: overviewSearch

                Layout.fillWidth: true
                app: page.app
            }

            SectionTiles {
                Layout.fillWidth: true
                Layout.bottomMargin: Kirigami.Units.gridUnit
                app: page.app
            }
        }

        Item {
            Layout.fillHeight: true
        }
    }
}
