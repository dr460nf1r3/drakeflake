pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../components"
import "../main/helpers.js" as Helpers

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var system: page.app.systemModel
    readonly property var modes: page.system.capabilities.generation_modes
    readonly property var generationsModel: page.app.generationsModel
    readonly property var generations: page.generationsModel.generations

    function reload(): void {
        page.generationsModel.load_generations();
    }

    function act(id: int, action: string): void {
        page.system.generation_action(id, action);
        page.app.showSection("log");
    }

    title: qsTr("Generations")

    actions: [
        Kirigami.Action {
            text: qsTr("Refresh")
            icon.name: "view-refresh"
            displayHint: Kirigami.DisplayHint.IconOnly
            enabled: !page.generationsModel.generations_loading

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

        model: page.generations

        header: ColumnLayout {
            width: ListView.view ? ListView.view.width : 0
            spacing: 0

            Controls.Label {
                Layout.fillWidth: true
                padding: Kirigami.Units.largeSpacing
                wrapMode: Text.Wrap
                opacity: 0.75
                text: qsTr("Every rebuild creates a generation. Test runs one until the next reboot, Boot makes it the default from the next boot on, Switch does both now. Compare lists what differs from the running system.")
            }

            Kirigami.InlineMessage {
                Layout.fillWidth: true
                Layout.margins: Kirigami.Units.smallSpacing
                visible: page.generations.length > 0 && !page.generations.some(generation => generation.running)
                type: Kirigami.MessageType.Information
                text: qsTr("The running system is not one of these generations (it was activated with Test). It is replaced by the default generation at the next boot.")
            }
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

                    Flow {
                        Layout.fillWidth: true
                        spacing: Kirigami.Units.smallSpacing

                        Controls.Label {
                            textFormat: Text.PlainText
                            text: qsTr("Generation %1").arg(row.modelData.id)
                            font.bold: true
                        }

                        Tag {
                            visible: row.modelData.running
                            text: row.modelData.running_specialisation ? qsTr("running (%1)").arg(row.modelData.running_specialisation) : qsTr("running")
                            tone: "positive"
                        }

                        Tag {
                            visible: row.modelData.current
                            text: qsTr("default at boot")
                            tone: "info"
                        }

                        Tag {
                            visible: row.modelData.booted && !row.modelData.running
                            text: qsTr("booted")
                        }
                    }

                    Controls.Label {
                        Layout.fillWidth: true
                        opacity: 0.75
                        wrapMode: Text.Wrap
                        textFormat: Text.PlainText
                        text: [Helpers.formatDate(row.modelData.date), row.modelData.version, row.modelData.kernel].filter(part => part.length > 0).join(" · ")
                    }

                    Controls.Label {
                        Layout.fillWidth: true
                        visible: row.modelData.specialisations.length > 0
                        opacity: 0.6
                        font: Kirigami.Theme.smallFont
                        wrapMode: Text.Wrap
                        textFormat: Text.PlainText
                        text: qsTr("Specialisations: %1").arg(row.modelData.specialisations.join(", "))
                    }
                }

                Flow {
                    Layout.maximumWidth: Kirigami.Units.gridUnit * 22
                    spacing: Kirigami.Units.smallSpacing
                    layoutDirection: Qt.RightToLeft

                    Controls.Button {
                        text: qsTr("Switch to")
                        icon.name: "system-switch-user"
                        icon.width: Kirigami.Units.iconSizes.sizeForLabels
                        icon.height: Kirigami.Units.iconSizes.sizeForLabels
                        enabled: !page.system.running && !row.modelData.running
                        visible: !row.modelData.running && page.modes.includes("switch")

                        onClicked: page.act(row.modelData.id, "switch")
                    }

                    Controls.Button {
                        text: qsTr("Boot")
                        icon.name: "system-reboot"
                        icon.width: Kirigami.Units.iconSizes.sizeForLabels
                        icon.height: Kirigami.Units.iconSizes.sizeForLabels
                        enabled: !page.system.running && !row.modelData.current
                        visible: !row.modelData.current && page.modes.includes("boot")

                        onClicked: page.act(row.modelData.id, "boot")
                    }

                    Controls.Button {
                        text: qsTr("Test")
                        icon.name: "media-playback-start"
                        icon.width: Kirigami.Units.iconSizes.sizeForLabels
                        icon.height: Kirigami.Units.iconSizes.sizeForLabels
                        enabled: !page.system.running && !row.modelData.running
                        visible: !row.modelData.running && page.modes.includes("test")

                        onClicked: page.act(row.modelData.id, "test")
                    }

                    Controls.Button {
                        text: qsTr("Compare")
                        icon.name: "vcs-diff"
                        icon.width: Kirigami.Units.iconSizes.sizeForLabels
                        icon.height: Kirigami.Units.iconSizes.sizeForLabels
                        enabled: !page.system.running && !row.modelData.running
                        visible: !row.modelData.running

                        onClicked: page.act(row.modelData.id, "compare")
                    }
                }
            }
        }

        Kirigami.PlaceholderMessage {
            anchors.centerIn: parent
            width: parent.width - Kirigami.Units.gridUnit * 4
            visible: list.count === 0
            icon.name: "view-history"
            text: page.generationsModel.generations_loading ? qsTr("Reading generations…") : qsTr("No system generations found")
            explanation: page.generationsModel.generations_loading ? "" : qsTr("They are read from /nix/var/nix/profiles.")
        }
    }
}
