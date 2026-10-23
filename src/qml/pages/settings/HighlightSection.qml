import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import org.kde.syntaxhighlighting

import "../../components"

SettingsSection {
    id: section

    required property var settingsModel
    required property string desktopHighlightTheme
    required property string highlightTheme
    readonly property real labelWidth: Math.max(themeLabel.implicitWidth, previewLabel.implicitWidth)

    SectionHeading {
        Layout.columnSpan: 2
        text: qsTr("Code highlighting")
    }

    FieldLabel {
        id: themeLabel

        Layout.minimumWidth: section.labelColumnWidth
        text: qsTr("Theme:")
    }

    Controls.ComboBox {
        id: themeBox

        function syncIndex(): void {
            themeBox.currentIndex = Math.max(0, themeBox.indexOfValue(section.settingsModel.highlight_theme));
        }

        Layout.minimumWidth: Kirigami.Units.gridUnit * 14
        model: [
            {
                value: "",
                text: section.desktopHighlightTheme.length > 0 ? qsTr("Automatic (%1, from your desktop theme)").arg(section.desktopHighlightTheme) : qsTr("Automatic (follow the colour scheme)")
            }
        ].concat(Repository.themes.map(theme => ({
                    value: theme.name,
                    text: theme.translatedName
                })))
        textRole: "text"
        valueRole: "value"

        Component.onCompleted: themeBox.syncIndex()
        onModelChanged: Qt.callLater(themeBox.syncIndex)
        onActivated: section.settingsModel.set_highlight_theme(themeBox.currentValue)

        Connections {
            function onState_changed(): void {
                if (themeBox.currentValue !== section.settingsModel.highlight_theme)
                    themeBox.syncIndex();
            }

            target: section.settingsModel
        }
    }

    FieldLabel {
        id: previewLabel

        Layout.minimumWidth: section.labelColumnWidth
        text: qsTr("Preview:")
    }

    CodeBlock {
        Layout.fillWidth: true
        themeName: section.highlightTheme
        // drakeflake-lint: allow(no-untranslated-text) Nix sample code, not prose.
        text: "# A comment\nservices.openssh = {\n  enable = true;\n  ports = [ 22 ];\n  settings.PermitRootLogin = \"no\";\n};"
    }
}
