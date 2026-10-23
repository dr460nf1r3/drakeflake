import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"

SettingsSection {
    id: section

    required property var settingsModel
    required property bool macos
    readonly property real labelWidth: blurLabel.implicitWidth

    visible: !section.macos

    SectionHeading {
        Layout.columnSpan: 2
        text: qsTr("Appearance")
    }

    FieldLabel {
        id: blurLabel

        Layout.minimumWidth: section.labelColumnWidth
        text: qsTr("Blurred background:")
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing

        Controls.ComboBox {
            id: blurBox

            function syncIndex(): void {
                blurBox.currentIndex = Math.max(0, blurBox.indexOfValue(section.settingsModel.blur));
            }

            Layout.minimumWidth: Kirigami.Units.gridUnit * 14
            model: [
                {
                    value: "auto",
                    text: qsTr("Automatic (blur on Wayland)")
                },
                {
                    value: "on",
                    text: qsTr("On")
                },
                {
                    value: "off",
                    text: qsTr("Off")
                }
            ]
            textRole: "text"
            valueRole: "value"

            Component.onCompleted: blurBox.syncIndex()
            onActivated: blurError.text = section.settingsModel.set_blur(blurBox.currentValue)

            Connections {
                function onState_changed(): void {
                    if (blurBox.currentValue !== section.settingsModel.blur)
                        blurBox.syncIndex();
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
            text: section.settingsModel.blur_env().length > 0 ? qsTr("DRAKEFLAKE_BLUR=%1 overrides this choice.").arg(section.settingsModel.blur_env()) : section.settingsModel.blur === "auto" ? qsTr("Automatic: the background blurs where the Wayland compositor offers it, and stays solid elsewhere (always on X11).") : section.settingsModel.blur_effective ? qsTr("The blur is on: the compositor blurs the desktop behind the window.") : qsTr("The window stays solid.")
        }

        Controls.Label {
            id: blurError

            Layout.fillWidth: true
            color: Kirigami.Theme.negativeTextColor
            wrapMode: Text.Wrap
        }
    }
}
