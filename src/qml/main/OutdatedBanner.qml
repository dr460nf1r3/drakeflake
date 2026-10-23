import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: banner

    required property bool outdated
    required property real contentX
    required property real windowWidth
    required property real topClearance

    visible: banner.outdated
    height: visible ? outdatedMessage.implicitHeight + banner.topClearance : 0

    Kirigami.InlineMessage {
        id: outdatedMessage

        x: banner.contentX
        y: banner.topClearance
        width: banner.windowWidth
        visible: true
        position: Kirigami.InlineMessage.Position.Header
        type: Kirigami.MessageType.Error
        text: qsTr("This program is older than its interface files, so some pages will not work. Rebuild it (cargo build) or start the matching version.")
    }
}
