import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../main/helpers.js" as Helpers

ColumnLayout {
    id: header

    required property var app
    required property string error
    required property string message
    required property bool updated
    required property bool busy
    required property bool checking
    required property int inputCount
    required property int updatableCount

    signal rebuildRequested(string mode)

    spacing: Kirigami.Units.smallSpacing

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: header.error.length > 0
        type: Kirigami.MessageType.Error
        text: Helpers.richText(header.error)
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: header.message.length > 0
        type: Kirigami.MessageType.Warning
        text: Helpers.richText(header.message)
        showCloseButton: true
    }

    Kirigami.InlineMessage {
        id: updatedMessage

        readonly property var rebuildModes: header.app.systemModel.capabilities.rebuild

        Layout.fillWidth: true
        visible: header.updated
        type: Kirigami.MessageType.Positive
        text: updatedMessage.rebuildModes.length > 0 ? qsTr("flake.lock was updated. Rebuild to start using the new versions.") : qsTr("flake.lock was updated. Rebuild on the machine this host runs on to use the new versions.")

        actions: [
            Kirigami.Action {
                text: qsTr("Rebuild and switch")
                icon.name: "system-run"
                enabled: !header.busy
                visible: updatedMessage.rebuildModes.includes("switch")

                onTriggered: header.rebuildRequested("switch")
            },
            Kirigami.Action {
                text: qsTr("Build only")
                icon.name: "run-build"
                enabled: !header.busy
                visible: updatedMessage.rebuildModes.includes("build")

                onTriggered: header.rebuildRequested("build")
            }
        ]
    }

    Controls.Label {
        Layout.fillWidth: true
        Layout.margins: Kirigami.Units.smallSpacing
        visible: header.inputCount > 0
        opacity: 0.75
        wrapMode: Text.Wrap
        text: {
            const count = header.inputCount === 1 ? qsTr("1 input") : qsTr("%1 inputs").arg(header.inputCount);

            if (header.checking)
                return qsTr("%1 · checking for updates…").arg(count);

            if (header.updatableCount > 0)
                return qsTr("%1 · %2 can be updated").arg(count).arg(header.updatableCount);

            return count;
        }
    }
}
