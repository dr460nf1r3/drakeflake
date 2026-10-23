import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"

SettingsSection {
    id: section

    required property var settingsModel
    readonly property real labelWidth: Math.max(layoutLabel.implicitWidth, rulesLabel.implicitWidth)

    SectionHeading {
        Layout.columnSpan: 2
        text: qsTr("Generated files")
    }

    FieldLabel {
        id: layoutLabel

        Layout.minimumWidth: section.labelColumnWidth
        text: qsTr("File layout:")
    }

    Controls.ComboBox {
        id: layoutBox

        model: [
            {
                value: "area-module",
                text: qsTr("Area and module (services/openssh.nix)")
            },
            {
                value: "area",
                text: qsTr("By area (services.nix)")
            },
            {
                value: "single",
                text: qsTr("Single file")
            }
        ]
        textRole: "text"
        valueRole: "value"

        // Set once: indexOfValue() needs model and valueRole in place, and the
        // selection is an unsaved edit until "Save layout".
        Component.onCompleted: layoutBox.currentIndex = Math.max(0, layoutBox.indexOfValue(section.settingsModel.layout_mode))
    }

    FieldLabel {
        id: rulesLabel

        Layout.minimumWidth: section.labelColumnWidth
        text: qsTr("Custom rules:")
    }

    MultiLineField {
        id: rulesField

        Layout.fillWidth: true
        Layout.minimumWidth: Kirigami.Units.gridUnit * 14
        font.family: "monospace"
        // drakeflake-lint: allow(no-untranslated-text) Nix sample code, not prose.
        placeholderText: "services.openssh = remote-access.nix\nenvironment.systemPackages = apps.nix"

        // Set once: a binding would overwrite the user's unsaved edits.
        Component.onCompleted: rulesField.text = section.settingsModel.layout_rules
    }

    Item {
    }

    Controls.Label {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        font: Kirigami.Theme.smallFont
        opacity: 0.75
        text: qsTr("One rule per line: settings under the prefix go into that file (in the folder next to the output file). Rules win over the layout. The longest matching prefix applies. Home Manager settings use the same layout inside home-manager/<user>/.")
    }

    Item {
    }

    RowLayout {
        Layout.fillWidth: true

        Controls.Button {
            text: qsTr("Save layout")
            icon.name: "document-save"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels

            onClicked: layoutError.text = section.settingsModel.set_layout(layoutBox.currentValue, rulesField.text)
        }

        Controls.Label {
            id: layoutError

            Layout.fillWidth: true
            color: Kirigami.Theme.negativeTextColor
            wrapMode: Text.Wrap
        }
    }
}
