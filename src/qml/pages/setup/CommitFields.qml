import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

ColumnLayout {
    id: fields

    required property bool autoCommit
    required property bool watchFlake

    signal autoCommitChosen(bool checked)
    signal watchFlakeChosen(bool checked)

    spacing: Kirigami.Units.smallSpacing

    Controls.CheckBox {
        id: commitBox

        text: qsTr("Commit the changes on apply")
        checked: fields.autoCommit

        onToggled: fields.autoCommitChosen(commitBox.checked)
    }

    Controls.Label {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        font: Kirigami.Theme.smallFont
        opacity: 0.75
        textFormat: Text.PlainText
        text: qsTr("Commits the flake after every successful apply, in a new git repository if it has none. You can change this later in Settings.")
    }

    Controls.CheckBox {
        id: watchBox

        text: qsTr("Evaluate in the background when the flake changes")
        checked: fields.watchFlake

        onToggled: fields.watchFlakeChosen(watchBox.checked)
    }

    Controls.Label {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        font: Kirigami.Theme.smallFont
        opacity: 0.75
        textFormat: Text.PlainText
        text: qsTr("Needs the tray icon. It re-reads the options and values after each change of the flake.")
    }
}
