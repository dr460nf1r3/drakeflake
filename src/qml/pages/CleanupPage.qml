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
    readonly property var generationsModel: page.app.generationsModel
    readonly property var info: page.generationsModel.cleanup_info
    readonly property bool reading: page.generationsModel.cleanup_loading && page.info.total === undefined
    readonly property real usedShare: Math.min(1, Math.max(0, ((page.info.total || 0) - (page.info.free || 0)) / (page.info.total || 1)))

    function reload(): void {
        page.generationsModel.load_cleanup_info();
    }

    title: qsTr("Cleanup")

    Component.onCompleted: page.reload()

    Connections {
        function onState_changed(): void {
            if (!page.system.running)
                page.reload();
        }

        target: page.system
    }

    GridLayout {
        columns: 2
        columnSpacing: Kirigami.Units.largeSpacing
        rowSpacing: Kirigami.Units.smallSpacing

        SectionHeading {
            Layout.columnSpan: 2
            Layout.topMargin: 0
            text: qsTr("Disk")
        }

        FieldLabel {
            text: qsTr("Nix store:")
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            // Used share of the disk. A plain meter: Breeze's ProgressBar logs
            // "Cannot read property 'top' of null" while it is resized.
            Rectangle {
                Layout.fillWidth: true
                Layout.maximumWidth: Kirigami.Units.gridUnit * 24
                Layout.preferredHeight: Kirigami.Units.smallSpacing * 2
                radius: height / 2
                color: Qt.alpha(Kirigami.Theme.textColor, 0.15)

                Rectangle {
                    width: parent.width * page.usedShare
                    height: parent.height
                    radius: parent.radius
                    // highlightColor renders near the background on macOS:
                    // a text tone stays readable on either appearance.
                    color: Qt.alpha(Kirigami.Theme.textColor, 0.6)
                }
            }

            Controls.Label {
                textFormat: Text.PlainText
                text: page.reading ? qsTr("Reading…") : qsTr("%1 free of %2 (%3% used)").arg(Helpers.gib(page.info.free)).arg(Helpers.gib(page.info.total)).arg(Math.round(page.usedShare * 100))
            }
        }

        FieldLabel {
            text: qsTr("Generations:")
        }

        Controls.Label {
            Layout.fillWidth: true
            textFormat: Text.PlainText
            text: page.reading ? qsTr("Reading…") : qsTr("%1 system generations").arg(page.info.generations || 0)
        }

        SectionHeading {
            Layout.columnSpan: 2
            text: qsTr("Clean up")
        }

        FieldLabel {
            text: qsTr("Keep at least:")
        }

        RowLayout {
            Controls.SpinBox {
                id: keepBox

                from: 1
                to: 100
                value: 5
            }

            Controls.Label {
                text: qsTr("newest generations")
            }
        }

        FieldLabel {
            text: qsTr("And everything from:")
        }

        RowLayout {
            Controls.SpinBox {
                id: daysBox

                from: 0
                to: 365
                value: 7
            }

            Controls.Label {
                text: qsTr("last days")
            }
        }

        Item {
        }

        Controls.CheckBox {
            id: optimiseBox

            text: qsTr("Also deduplicate the store (slow, saves space)")
        }

        Item {
        }

        Controls.Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            font: Kirigami.Theme.smallFont
            opacity: 0.75
            text: qsTr("Removes older generations, then everything nothing uses any more (nh clean all). It lists what goes and asks before deleting. Old boot menu entries disappear with the next rebuild.")
        }

        Item {
        }

        Flow {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            Controls.Button {
                text: qsTr("Clean up…")
                icon.name: "edit-clear-all"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels
                highlighted: true
                enabled: !page.system.running

                onClicked: {
                    page.system.clean_up(keepBox.value, daysBox.value, optimiseBox.checked);
                    page.app.showSection("log");
                }
            }

            Controls.Button {
                text: qsTr("Remove tried packages (%1)").arg(page.info.tried || 0)
                icon.name: "edit-delete"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels
                enabled: (page.info.tried || 0) > 0

                onClicked: {
                    page.generationsModel.remove_tried();
                    page.reload();
                }
            }
        }
    }
}
