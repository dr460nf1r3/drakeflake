import QtQuick
import org.kde.kirigami as Kirigami

Rectangle {
    id: background

    property bool selected: false
    property bool hovered: false

    color: selected ? Qt.alpha(Kirigami.Theme.highlightColor, 0.16) : hovered ? Qt.alpha(Kirigami.Theme.highlightColor, 0.07) : "transparent"

    Rectangle {
        objectName: "accent"
        visible: background.selected
        width: Math.round(Kirigami.Units.smallSpacing * 0.75)
        height: background.height
        color: Kirigami.Theme.highlightColor
    }
}
