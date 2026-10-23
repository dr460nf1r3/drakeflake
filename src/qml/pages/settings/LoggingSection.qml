import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"

SettingsSection {
    id: section

    required property var settingsModel
    readonly property real labelWidth: logLabel.implicitWidth

    SectionHeading {
        Layout.columnSpan: 2
        text: qsTr("Troubleshooting")
    }

    FieldLabel {
        id: logLabel

        Layout.minimumWidth: section.labelColumnWidth
        text: qsTr("Log detail:")
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing

        Controls.ComboBox {
            id: logLevelBox

            function syncIndex(): void {
                logLevelBox.currentIndex = Math.max(0, logLevelBox.indexOfValue(section.settingsModel.log_level));
            }

            Layout.minimumWidth: Kirigami.Units.gridUnit * 14
            model: [
                {
                    value: "error",
                    text: qsTr("Errors only")
                },
                {
                    value: "warn",
                    text: qsTr("Warnings")
                },
                {
                    value: "info",
                    text: qsTr("Normal (recommended)")
                },
                {
                    value: "debug",
                    text: qsTr("Debug: every command run")
                },
                {
                    value: "trace",
                    text: qsTr("Trace: also full command output")
                }
            ]
            textRole: "text"
            valueRole: "value"

            Component.onCompleted: logLevelBox.syncIndex()
            onActivated: section.settingsModel.set_log_level(logLevelBox.currentValue)

            Connections {
                function onState_changed(): void {
                    if (logLevelBox.currentValue !== section.settingsModel.log_level)
                        logLevelBox.syncIndex();
                }

                target: section.settingsModel
            }
        }

        Controls.Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            font: Kirigami.Theme.smallFont
            opacity: 0.75
            textFormat: Text.PlainText
            text: qsTr("Everything the app does is written to %1 (older logs are kept as .1 to .3). Debug messages from the interface itself are added after a restart.").arg(section.settingsModel.log_file())
        }

        RowLayout {
            spacing: Kirigami.Units.smallSpacing

            Controls.Button {
                text: qsTr("Open log")
                icon.name: "document-open"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels

                onClicked: section.settingsModel.open_log()
            }

            Controls.Button {
                text: qsTr("Show folder")
                icon.name: "folder-open"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels

                onClicked: section.settingsModel.show_log_folder()
            }
        }
    }
}
