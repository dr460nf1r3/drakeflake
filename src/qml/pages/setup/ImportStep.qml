import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers
import "../../main/kinds.js" as Kinds

ColumnLayout {
    id: step

    required property var setupModel
    required property string host
    required property bool targetReady
    required property string highlightTheme

    spacing: Kirigami.Units.largeSpacing

    Controls.Label {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        textFormat: Text.MarkdownText
        text: qsTr("Add this import to the configuration of **%1**, for example in its `configuration.nix`. If it already has an `imports` list, add the path to it. Paths are relative to the file that contains the import. This is the path from the top folder of the flake:").arg(step.host)
    }

    CodeBlock {
        Layout.fillWidth: true
        themeName: step.highlightTheme
        text: step.targetReady ? step.setupModel.import_line() : ""
    }

    Controls.Label {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        opacity: 0.75
        font: Kirigami.Theme.smallFont
        text: qsTr("Check import writes the generated file if it does not exist yet, adds it to git (flakes only see tracked files) and evaluates your host to see whether it imports the file.")
    }

    RowLayout {
        spacing: Kirigami.Units.largeSpacing

        Controls.Button {
            text: qsTr("Check import")
            icon.name: "dialog-ok"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels
            highlighted: step.setupModel.import_status !== Kinds.importImported
            enabled: step.targetReady && step.setupModel.import_status !== Kinds.importChecking

            onClicked: step.setupModel.check_import()
        }

        Controls.BusyIndicator {
            visible: step.setupModel.import_status === Kinds.importChecking
            running: visible
            Layout.preferredHeight: Kirigami.Units.iconSizes.sizeForLabels
            Layout.preferredWidth: Kirigami.Units.iconSizes.sizeForLabels
        }

        Controls.Label {
            visible: step.setupModel.import_status === Kinds.importChecking
            text: qsTr("Evaluating %1…").arg(step.host)
        }
    }

    Kirigami.InlineMessage {
        objectName: "importedMessage"
        Layout.fillWidth: true
        visible: step.setupModel.import_status === Kinds.importImported
        type: Kirigami.MessageType.Positive
        text: qsTr("%1 imports the generated file. You are all set.").arg(step.host)
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: step.setupModel.import_status === Kinds.importMissing
        type: Kirigami.MessageType.Warning
        text: qsTr("%1 does not import the generated file yet. Add the line above, save the file and check again.").arg(step.host)
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: step.setupModel.import_status === Kinds.importFailed
        type: Kirigami.MessageType.Error
        text: Helpers.richText(qsTr("The check failed: %1").arg(step.setupModel.import_error))
    }
}
