pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers

GridLayout {
    id: step

    required property bool scanning
    required property var candidates
    required property string flakePath
    required property string flakeRoot
    required property bool flakeLoaded
    required property bool loading
    required property bool targetReady
    required property string targetError
    required property var hostList
    required property string defaultFlake
    readonly property var hostNames: step.hostList.map(host => host.name)
    required property string host
    required property string folder
    required property bool folderExists
    required property bool folderInFlake
    required property bool folderIsFlakeRoot
    required property string folderError
    required property string layoutMode
    required property bool autoCommit
    required property bool watchFlake
    readonly property bool ready: step.targetReady && step.flakeLoaded

    signal flakePathChosen(string path)
    signal browseFlake
    signal hostChosen(string host)
    signal folderEdited(string text)
    signal browseFolder
    signal createFolder
    signal layoutChosen(string mode)
    signal autoCommitChosen(bool checked)
    signal watchFlakeChosen(bool checked)

    columns: 2
    columnSpacing: Kirigami.Units.largeSpacing
    rowSpacing: Kirigami.Units.smallSpacing

    SectionHeading {
        Layout.columnSpan: 2
        Layout.topMargin: 0
        text: qsTr("System flake")
    }

    FieldLabel {
        visible: step.scanning
        text: qsTr("Found:")
    }

    RowLayout {
        visible: step.scanning

        Controls.BusyIndicator {
            Layout.preferredWidth: Kirigami.Units.iconSizes.sizeForLabels
            Layout.preferredHeight: Kirigami.Units.iconSizes.sizeForLabels
            running: step.scanning
        }

        Controls.Label {
            text: qsTr("Looking for system flakes…")
        }
    }

    FieldLabel {
        visible: step.candidates.length > 0
        text: qsTr("Found:")
    }

    ColumnLayout {
        Layout.fillWidth: true
        visible: step.candidates.length > 0
        spacing: 0

        Repeater {
            model: step.candidates

            delegate: Controls.RadioButton {
                required property string modelData

                text: modelData
                checked: step.flakePath === modelData

                onToggled: {
                    if (checked)
                        step.flakePathChosen(modelData);
                }
            }
        }
    }

    FieldLabel {
        text: qsTr("Folder:")
    }

    RowLayout {
        Layout.fillWidth: true

        Controls.TextField {
            id: flakeField

            Layout.fillWidth: true
            text: step.flakePath
            placeholderText: step.defaultFlake

            onTextEdited: step.flakePathChosen(flakeField.text.trim())
        }

        Controls.Button {
            icon.name: "document-open-folder"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels
            text: qsTr("Browse…")

            onClicked: step.browseFlake()
        }
    }

    Item {
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: step.flakePath.length > 0 && step.flakeRoot.length === 0
        type: Kirigami.MessageType.Warning
        text: qsTr("There is no flake.nix in this folder or above it.")
    }

    SectionHeading {
        Layout.columnSpan: 2
        visible: step.flakeLoaded
        text: qsTr("Host")
    }

    Item {
        visible: step.loading
    }

    RowLayout {
        visible: step.loading

        Controls.BusyIndicator {
            Layout.preferredWidth: Kirigami.Units.iconSizes.sizeForLabels
            Layout.preferredHeight: Kirigami.Units.iconSizes.sizeForLabels
            running: step.loading
        }

        Controls.Label {
            text: qsTr("Reading the hosts of the flake…")
        }
    }

    Item {
        visible: step.flakeLoaded && step.targetError.length > 0
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: step.flakeLoaded && step.targetError.length > 0
        type: Kirigami.MessageType.Error
        text: Helpers.richText(step.targetError)
    }

    FieldLabel {
        visible: step.flakeLoaded && step.hostList.length > 0
        text: qsTr("Host:")
    }

    Controls.ComboBox {
        id: hostBox

        Layout.minimumWidth: Kirigami.Units.gridUnit * 14
        visible: step.flakeLoaded && step.hostList.length > 0
        model: step.hostNames
        currentIndex: Math.max(0, step.hostNames.indexOf(step.host))

        onActivated: step.hostChosen(hostBox.currentText)
    }

    SectionHeading {
        Layout.columnSpan: 2
        visible: step.ready
        text: qsTr("Folder for the generated files")
    }

    Controls.Label {
        Layout.columnSpan: 2
        Layout.fillWidth: true
        visible: step.ready
        wrapMode: Text.Wrap
        // A default.nix in the flake's own folder would be its entry point, not an importable module.
        text: {
            if (step.layoutMode === "single")
                return qsTr("The settings are written to drakeflake.nix in this folder. It must be inside the flake.");
            if (step.folderIsFlakeRoot)
                return qsTr("The settings are written to drakeflake.nix in this folder, with their parts in the drakeflake folder next to it. It must be inside the flake.");
            return qsTr("The settings are written to default.nix in this folder, with their parts next to it, so the folder itself is imported. It must be inside the flake.");
        }
    }

    FieldLabel {
        visible: step.ready
        text: qsTr("Folder:")
    }

    RowLayout {
        Layout.fillWidth: true
        visible: step.ready

        Controls.TextField {
            id: folderField

            Layout.fillWidth: true
            text: step.folder
            placeholderText: step.flakeRoot

            onTextEdited: step.folderEdited(folderField.text)
        }

        Controls.Button {
            icon.name: "document-open-folder"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels
            text: qsTr("Browse…")

            onClicked: step.browseFolder()
        }

        Controls.Button {
            visible: !step.folderExists && step.folderInFlake
            icon.name: "folder-new"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels
            text: qsTr("Create folder")

            onClicked: step.createFolder()
        }
    }

    Item {
        visible: folderMessage.visible
    }

    Kirigami.InlineMessage {
        id: folderMessage

        Layout.fillWidth: true
        visible: step.ready && (step.folderError.length > 0 || !step.folderInFlake || !step.folderExists)
        type: step.folderError.length > 0 || !step.folderInFlake ? Kirigami.MessageType.Error : Kirigami.MessageType.Warning
        text: Helpers.richText(step.folderError.length > 0 ? step.folderError : !step.folderInFlake ? qsTr("The folder must be inside the flake (%1).").arg(step.flakeRoot) : qsTr("This folder does not exist yet."))
    }

    SectionHeading {
        Layout.columnSpan: 2
        visible: step.ready
        text: qsTr("File layout")
    }

    FieldLabel {
        visible: step.ready
        text: qsTr("Layout:")
    }

    ColumnLayout {
        Layout.fillWidth: true
        visible: step.ready
        spacing: Kirigami.Units.smallSpacing

        Repeater {
            model: [
                {
                    value: "area-module",
                    text: qsTr("Area and module"),
                    example: "drakeflake.nix + drakeflake/services/openssh.nix, drakeflake/networking/firewall.nix"
                },
                {
                    value: "area",
                    text: qsTr("By area"),
                    example: "drakeflake.nix + drakeflake/services.nix, drakeflake/networking.nix"
                },
                {
                    value: "single",
                    text: qsTr("Single file"),
                    example: "drakeflake.nix"
                }
            ]

            delegate: ColumnLayout {
                id: layoutChoice

                required property var modelData

                Layout.fillWidth: true
                spacing: 0

                Controls.RadioButton {
                    text: layoutChoice.modelData.text
                    checked: step.layoutMode === layoutChoice.modelData.value

                    onToggled: {
                        if (checked)
                            step.layoutChosen(layoutChoice.modelData.value);
                    }
                }

                Controls.Label {
                    Layout.fillWidth: true
                    Layout.leftMargin: Kirigami.Units.gridUnit * 2
                    wrapMode: Text.WrapAnywhere
                    font.family: "monospace"
                    font.pointSize: Kirigami.Theme.smallFont.pointSize
                    opacity: 0.7
                    textFormat: Text.PlainText
                    text: layoutChoice.modelData.example
                }
            }
        }

        Controls.Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            font: Kirigami.Theme.smallFont
            opacity: 0.75
            text: qsTr("You can change this and add your own rules later in Settings.")
        }
    }

    SectionHeading {
        Layout.columnSpan: 2
        visible: step.ready
        text: qsTr("Committing")
    }

    FieldLabel {
        visible: step.ready
        text: qsTr("Committing:")
    }

    CommitFields {
        Layout.fillWidth: true
        visible: step.ready
        autoCommit: step.autoCommit
        watchFlake: step.watchFlake

        onAutoCommitChosen: checked => step.autoCommitChosen(checked)
        onWatchFlakeChosen: checked => step.watchFlakeChosen(checked)
    }
}
