pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"

SettingsSection {
    id: section

    required property var settingsModel
    required property bool macos
    readonly property var tray: section.settingsModel.tray_settings
    readonly property real labelWidth: Math.max(trayLabel.implicitWidth, updatesLabel.implicitWidth, watchLabel.implicitWidth, reportsLabel.implicitWidth)

    SectionHeading {
        Layout.columnSpan: 2
        text: qsTr("Update notifications")
    }

    FieldLabel {
        id: trayLabel

        Layout.minimumWidth: section.labelColumnWidth
        text: section.macos ? qsTr("Menu bar icon:") : qsTr("Tray icon:")
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing

        Controls.CheckBox {
            id: trayBox

            text: qsTr("Start at login")
            checked: section.settingsModel.tray_autostart

            onToggled: section.settingsModel.set_tray_autostart(trayBox.checked)
        }

        Controls.Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            font: Kirigami.Theme.smallFont
            opacity: 0.75
            text: section.macos ? qsTr("A small menu bar icon (not the whole app) checks your flake inputs and this machine, and notifies you when something needs doing.") : qsTr("A small tray icon (not the whole app) checks your flake inputs and this machine, and notifies you when something needs doing.")
        }

        Controls.Button {
            text: qsTr("Start it now")
            icon.name: "media-playback-start"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels

            onClicked: section.settingsModel.start_tray()
        }
    }

    FieldLabel {
        id: updatesLabel

        Layout.minimumWidth: section.labelColumnWidth
        text: qsTr("Updates:")
    }

    RowLayout {
        spacing: Kirigami.Units.smallSpacing

        Controls.CheckBox {
            id: updatesBox

            text: qsTr("Check the flake inputs every")
            checked: section.tray.updates

            onToggled: section.settingsModel.set_tray_report("updates", updatesBox.checked)
        }

        Controls.SpinBox {
            id: hoursBox

            enabled: updatesBox.checked
            from: 1
            to: 168
            value: section.tray.check_hours
            editable: true

            onValueModified: section.settingsModel.set_tray_interval("check_hours", hoursBox.value)
        }

        Controls.Label {
            text: hoursBox.value === 1 ? qsTr("hour") : qsTr("hours")
        }
    }

    FieldLabel {
        id: watchLabel

        Layout.minimumWidth: section.labelColumnWidth
        text: qsTr("Flake changes:")
    }

    Controls.CheckBox {
        id: watchBox

        text: qsTr("Evaluate the flake in the background when its files change")
        checked: section.tray.watch_flake === true

        onToggled: section.settingsModel.set_tray_report("watch_flake", watchBox.checked)
    }

    FieldLabel {
        id: reportsLabel

        Layout.alignment: Qt.AlignTop
        Layout.minimumWidth: section.labelColumnWidth
        text: qsTr("Also report:")
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing

        Repeater {
            model: [
                {
                    name: "restart",
                    label: qsTr("A restart is needed for a new kernel")
                },
                {
                    name: "failed_services",
                    label: qsTr("System services that failed")
                },
                {
                    name: "low_space",
                    label: qsTr("Little disk space left for Nix")
                },
                {
                    name: "staged",
                    label: qsTr("Changes staged here and not applied")
                }
            ]

            delegate: Controls.CheckBox {
                id: reportBox

                required property var modelData

                text: reportBox.modelData.label
                checked: section.tray[reportBox.modelData.name] === true

                onToggled: section.settingsModel.set_tray_report(reportBox.modelData.name, reportBox.checked)
            }
        }

        RowLayout {
            spacing: Kirigami.Units.smallSpacing

            Controls.Label {
                text: qsTr("Look every")
            }

            Controls.SpinBox {
                id: minutesBox

                from: 1
                to: 60
                value: section.tray.health_minutes
                editable: true

                onValueModified: section.settingsModel.set_tray_interval("health_minutes", minutesBox.value)
            }

            Controls.Label {
                text: minutesBox.value === 1 ? qsTr("minute") : qsTr("minutes")
            }
        }
    }
}
