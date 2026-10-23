import QtQuick
import org.kde.kirigami as Kirigami

Rectangle {
    id: block

    property alias text: content.text
    property int maximumLines: 0
    property bool highlightNix: true
    property string themeName: ""

    implicitHeight: content.implicitHeight + Kirigami.Units.smallSpacing * 4
    radius: Kirigami.Units.cornerRadius
    color: highlighter.hasColors ? highlighter.backgroundColor : Kirigami.Theme.alternateBackgroundColor
    border.width: 1
    border.color: Qt.alpha(Kirigami.Theme.textColor, 0.12)

    TextEdit {
        id: content

        objectName: "content"
        readOnly: true
        selectByMouse: true
        wrapMode: TextEdit.WrapAnywhere
        textFormat: TextEdit.PlainText
        color: highlighter.hasColors ? highlighter.textColor : Kirigami.Theme.textColor
        selectionColor: Kirigami.Theme.highlightColor
        selectedTextColor: Kirigami.Theme.highlightedTextColor

        anchors {
            fill: parent
            margins: Kirigami.Units.smallSpacing * 2
        }

        font {
            family: "monospace"
            pointSize: Kirigami.Theme.defaultFont.pointSize
        }

        NixHighlighter {
            id: highlighter

            objectName: "highlighter"
            textEdit: block.highlightNix ? content : null
            themeName: block.themeName
            dark: Kirigami.ColorUtils.brightnessForColor(block.Kirigami.Theme.backgroundColor) === Kirigami.ColorUtils.Dark
        }
    }
}
