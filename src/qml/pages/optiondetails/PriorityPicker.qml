pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"

RowLayout {
    id: picker

    required property var choices
    required property string implicitChoice
    required property string shownPriority

    signal picked(string priority)

    spacing: Kirigami.Units.largeSpacing

    SettingText {
        id: settingText
    }

    Controls.ComboBox {
        id: priorityBox

        objectName: "priorityBox"
        Layout.minimumWidth: Kirigami.Units.gridUnit * 12
        model: picker.choices
        currentIndex: picker.choices.indexOf(picker.shownPriority)
        displayText: settingText.priorityLabel(priorityBox.currentText)

        delegate: Controls.ItemDelegate {
            id: choice

            required property string modelData
            required property int index

            width: ListView.view ? ListView.view.width : implicitWidth
            highlighted: priorityBox.highlightedIndex === choice.index

            contentItem: ColumnLayout {
                spacing: 0

                Controls.Label {
                    Layout.fillWidth: true
                    textFormat: Text.PlainText
                    text: settingText.priorityLabel(choice.modelData)
                }

                Controls.Label {
                    Layout.fillWidth: true
                    textFormat: Text.PlainText
                    text: settingText.priorityHint(choice.modelData)
                    font: Kirigami.Theme.smallFont
                    opacity: 0.7
                    wrapMode: Text.Wrap
                }
            }
        }

        onActivated: index => {
            const chosen = picker.choices[index];
            picker.picked(chosen === picker.implicitChoice ? "" : chosen);
        }
    }

    Controls.Label {
        Layout.fillWidth: true
        text: settingText.priorityHint(picker.shownPriority)
        wrapMode: Text.Wrap
        font: Kirigami.Theme.smallFont
        opacity: 0.75
    }
}
