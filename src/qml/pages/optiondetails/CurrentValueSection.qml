import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers
import "../../main/kinds.js" as Kinds

ColumnLayout {
    id: section

    required property var app
    required property var info

    signal evaluateRequested
    signal openLocation(string file, int line)

    spacing: Kirigami.Units.largeSpacing

    SectionHeading {
        text: qsTr("Current value")
    }

    RowLayout {
        id: loading

        Layout.fillWidth: true
        visible: section.info.valueState === Kinds.valueLoading
        spacing: Kirigami.Units.smallSpacing

        Controls.BusyIndicator {
            running: loading.visible
            Layout.preferredWidth: Kirigami.Units.iconSizes.sizeForLabels
            Layout.preferredHeight: Kirigami.Units.iconSizes.sizeForLabels
        }

        Controls.Label {
            Layout.fillWidth: true
            text: qsTr("Evaluating your configuration…")
            opacity: 0.7
        }
    }

    CodeBlock {
        Layout.fillWidth: true
        themeName: section.app.highlightTheme
        visible: section.info.valueState === Kinds.valueReady
        text: section.info.valueDisplay || ""
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: section.info.valueState === Kinds.valueError
        type: Kirigami.MessageType.Warning
        text: Helpers.richText(section.info.valueError || "")

        actions: [
            Kirigami.Action {
                text: qsTr("Try again")
                icon.name: "view-refresh"

                onTriggered: section.evaluateRequested()
            }
        ]
    }

    Controls.Button {
        visible: section.info.valueState === Kinds.valueUnknown
        text: qsTr("Evaluate current value")
        icon.name: "run-build"
        icon.width: Kirigami.Units.iconSizes.sizeForLabels
        icon.height: Kirigami.Units.iconSizes.sizeForLabels

        onClicked: section.evaluateRequested()
    }

    LocationList {
        id: locationList

        Layout.fillWidth: true
        visible: locationList.locations.length > 0
        app: section.app
        locations: section.info.locations || []

        onOpened: (file, line) => section.openLocation(file, line)
    }
}
