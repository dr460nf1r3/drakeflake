import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"

SettingsSection {
    id: section

    required property var settingsModel
    readonly property real labelWidth: commitLabel.implicitWidth

    SectionHeading {
        Layout.columnSpan: 2
        Layout.topMargin: 0
        text: qsTr("Applying")
    }

    FieldLabel {
        id: commitLabel

        Layout.minimumWidth: section.labelColumnWidth
        text: qsTr("Committing:")
    }

    ColumnLayout {
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing

        Controls.CheckBox {
            id: commitBox

            text: qsTr("Commit the changes on apply")
            checked: section.settingsModel.auto_commit

            onToggled: section.settingsModel.set_auto_commit(commitBox.checked)
        }

        Controls.Label {
            Layout.fillWidth: true
            wrapMode: Text.Wrap
            font: Kirigami.Theme.smallFont
            opacity: 0.75
            textFormat: Text.PlainText
            text: qsTr("Commits the flake after every successful apply, in a new git repository if it has none.")
        }
    }
}
