import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

// Used instead of Controls.TextArea, whose Breeze implementation logs a warning on creation.
Rectangle {
    id: field

    property alias text: edit.text
    property alias font: edit.font
    property string placeholderText: ""
    readonly property bool editing: edit.activeFocus

    signal textEdited(string text)

    implicitHeight: Math.max(Kirigami.Units.gridUnit * 5, edit.implicitHeight + Kirigami.Units.smallSpacing * 2)
    radius: Kirigami.Units.cornerRadius
    color: enabled ? Kirigami.Theme.backgroundColor : Qt.darker(Kirigami.Theme.backgroundColor, 1.3)
    border.width: 1
    border.color: edit.activeFocus ? Kirigami.Theme.highlightColor : Qt.alpha(Kirigami.Theme.textColor, 0.25)
    opacity: enabled ? 1 : 0.6
    Kirigami.Theme.colorSet: Kirigami.Theme.View
    Kirigami.Theme.inherit: false

    TextEdit {
        id: edit

        anchors.fill: parent
        anchors.margins: Kirigami.Units.smallSpacing * 2
        wrapMode: TextEdit.Wrap
        selectByMouse: true
        textFormat: TextEdit.PlainText
        color: Kirigami.Theme.textColor
        selectionColor: Kirigami.Theme.highlightColor
        selectedTextColor: Kirigami.Theme.highlightedTextColor
        font.family: Kirigami.Theme.defaultFont.family

        // TextEdit has no textEdited signal. activeFocus filters out programmatic changes.
        onTextChanged: if (edit.activeFocus)
            field.textEdited(edit.text)
    }

    FieldMenu {
        target: edit
    }

    Controls.Label {
        objectName: "placeholder"
        anchors.fill: edit
        visible: edit.text.length === 0 && !edit.activeFocus
        text: field.placeholderText
        opacity: 0.5
        font: edit.font
        wrapMode: Text.Wrap
    }

    TapHandler {
        onTapped: edit.forceActiveFocus()
    }
}
