import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers

SettingsSection {
    id: section

    required property var system
    required property var setupModel
    readonly property var hostList: section.system.hosts
    readonly property var hostNames: section.hostList.map(host => host.name)
    readonly property string defaultFlake: section.system.capabilities.default_flake
    readonly property real labelWidth: Math.max(flakeLabel.implicitWidth, hostLabel.implicitWidth, outputLabel.implicitWidth, writesToLabel.implicitWidth)
    readonly property string startFolder: flakeField.text.trim() || section.defaultFlake

    signal browseFlake
    signal chooseOutputFile
    signal chooseOutputFolder
    signal setupRequested

    function setFlake(path: string): void {
        flakeField.text = path;
    }

    function setOutput(path: string): void {
        outputField.text = path;
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        Layout.columnSpan: 2
        visible: section.system.target_error.length > 0
        type: Kirigami.MessageType.Error
        text: Helpers.richText(section.system.target_error)
    }

    SectionHeading {
        Layout.columnSpan: 2
        Layout.topMargin: 0
        text: qsTr("System")
    }

    FieldLabel {
        id: flakeLabel

        Layout.minimumWidth: section.labelColumnWidth
        text: qsTr("System flake:")
    }

    RowLayout {
        Layout.fillWidth: true

        Controls.TextField {
            id: flakeField

            Layout.fillWidth: true
            Layout.minimumWidth: Kirigami.Units.gridUnit * 14
            text: section.system.flake
            placeholderText: section.defaultFlake
            font.family: "monospace"

            onAccepted: apply.clicked()
        }

        Controls.Button {
            id: browseButton

            icon.name: "document-open-folder"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels
            text: qsTr("Browse…")
            display: Controls.AbstractButton.IconOnly
            Controls.ToolTip.visible: browseButton.hovered
            Controls.ToolTip.text: qsTr("Choose the folder containing flake.nix")

            onClicked: section.browseFlake()
        }
    }

    FieldLabel {
        id: hostLabel

        Layout.minimumWidth: section.labelColumnWidth
        text: section.system.kind.length > 0 ? qsTr("Host (%1):").arg(section.system.kind) : qsTr("Host:")
    }

    Controls.ComboBox {
        id: hostBox

        Layout.minimumWidth: Kirigami.Units.gridUnit * 14
        editable: true
        model: section.hostNames
        editText: section.system.host

        // The ComboBox overwrites editText with the first entry when it completes or gets a new model.
        Component.onCompleted: hostBox.editText = section.system.host
        onModelChanged: {
            if (!hostBox.activeFocus)
                hostBox.editText = section.system.host;
        }
    }

    FieldLabel {
        id: outputLabel

        Layout.minimumWidth: section.labelColumnWidth
        text: qsTr("Output file:")
    }

    RowLayout {
        Layout.fillWidth: true

        Controls.TextField {
            id: outputField

            Layout.fillWidth: true
            Layout.minimumWidth: Kirigami.Units.gridUnit * 14
            text: section.system.managed_setting
            placeholderText: qsTr("%1/drakeflake.nix (default)").arg(section.setupModel.normalize_folder(section.startFolder))
            font.family: "monospace"

            onAccepted: apply.clicked()
        }

        Controls.Button {
            id: chooseFileButton

            icon.name: "document-save-as"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels
            text: qsTr("Choose file…")
            display: Controls.AbstractButton.IconOnly
            Controls.ToolTip.visible: chooseFileButton.hovered
            Controls.ToolTip.text: qsTr("Choose the .nix file the app writes")

            onClicked: section.chooseOutputFile()
        }

        Controls.Button {
            id: chooseFolderButton

            icon.name: "document-open-folder"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels
            text: qsTr("Choose folder…")
            display: Controls.AbstractButton.IconOnly
            Controls.ToolTip.visible: chooseFolderButton.hovered
            Controls.ToolTip.text: qsTr("Choose a folder. The app writes default.nix inside it (drakeflake.nix with a single-file layout)")

            onClicked: section.chooseOutputFolder()
        }

        Controls.Button {
            id: defaultOutputButton

            icon.name: "edit-clear"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels
            text: qsTr("Use default")
            display: Controls.AbstractButton.IconOnly
            enabled: outputField.text.length > 0
            Controls.ToolTip.visible: defaultOutputButton.hovered
            Controls.ToolTip.text: qsTr("Write drakeflake.nix next to flake.nix")

            onClicked: outputField.text = ""
        }
    }

    FieldLabel {
        id: writesToLabel

        Layout.minimumWidth: section.labelColumnWidth
        visible: writesTo.visible
        text: qsTr("Writes to:")
    }

    Controls.Label {
        id: writesTo

        Layout.fillWidth: true
        visible: section.system.managed_path.length > 0
        text: section.system.managed_path
        font.family: "monospace"
        wrapMode: Text.WrapAnywhere
        opacity: 0.75
    }

    Item {
    }

    Flow {
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing

        Controls.Button {
            id: apply

            text: qsTr("Use this system")
            icon.name: "dialog-ok-apply"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels
            highlighted: true
            enabled: flakeField.text.trim().length > 0

            onClicked: section.system.retarget(flakeField.text, hostBox.editText, outputField.text)
        }

        Controls.Button {
            text: qsTr("Reload data")
            icon.name: "view-refresh"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels

            onClicked: section.system.reload()
        }

        Controls.Button {
            id: rebuildButton

            text: qsTr("Rebuild caches")
            icon.name: "edit-clear-history"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels
            Controls.ToolTip.visible: rebuildButton.hovered
            Controls.ToolTip.text: qsTr("Forget cached option, value and package indexes and evaluate everything again")

            onClicked: section.system.rebuild_caches()
        }

        Controls.Button {
            text: qsTr("Setup assistant…")
            icon.name: "tools-wizard"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels

            onClicked: section.setupRequested()
        }
    }
}
