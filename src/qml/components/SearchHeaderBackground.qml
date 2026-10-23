import QtQuick
import org.kde.kirigami as Kirigami

Rectangle {
    id: background

    required property bool clear

    color: background.clear ? "transparent" : Kirigami.Theme.backgroundColor

    Kirigami.Separator {
        anchors {
            left: parent.left
            right: parent.right
            bottom: parent.bottom
        }
    }
}
