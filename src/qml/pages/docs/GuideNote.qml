import QtQuick
import org.kde.kirigami as Kirigami

Rectangle {
    id: note

    required property bool warning
    property alias text: content.text
    readonly property color accent: note.warning ? Kirigami.Theme.neutralTextColor : Kirigami.Theme.activeTextColor

    signal linkActivated(string link)

    implicitHeight: content.implicitHeight + Kirigami.Units.largeSpacing * 2
    radius: Kirigami.Units.cornerRadius
    color: Qt.alpha(note.accent, 0.12)
    border.width: 1
    border.color: Qt.alpha(note.accent, 0.6)

    GuideText {
        id: content

        objectName: "noteText"

        onLinkActivated: link => note.linkActivated(link)

        anchors {
            fill: parent
            margins: Kirigami.Units.largeSpacing
        }
    }
}
