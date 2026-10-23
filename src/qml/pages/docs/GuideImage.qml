import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: frame

    required property url source
    required property string alt
    required property int preferredWidth
    required property int naturalWidth
    required property int naturalHeight
    readonly property bool ready: picture.status === Image.Ready
    readonly property real fullWidth: preferredWidth > 0 ? preferredWidth : (picture.implicitWidth > 0 ? picture.implicitWidth : naturalWidth)

    implicitHeight: picture.height

    Image {
        id: picture

        objectName: "guidePicture"
        source: frame.source
        fillMode: Image.PreserveAspectFit
        mipmap: true
        // Decoded off the interface thread, so long pages keep scrolling.
        asynchronous: true
        width: Math.min(frame.width, frame.fullWidth)

        // The parser's size reserves the space, so the text below does not jump when the image arrives.
        height: frame.naturalWidth > 0 && frame.naturalHeight > 0 ? picture.width * frame.naturalHeight / frame.naturalWidth : (picture.implicitWidth > 0 ? picture.width * picture.implicitHeight / picture.implicitWidth : 0)
        opacity: frame.ready ? 1 : 0
        Accessible.name: frame.alt
        Accessible.role: Accessible.Graphic

        Behavior on opacity {
            NumberAnimation {
                duration: 150
            }
        }
    }

    Rectangle {
        anchors.fill: picture
        visible: frame.preferredWidth === 0
        color: "transparent"
        radius: Kirigami.Units.cornerRadius
        border.width: 1
        border.color: Qt.alpha(Kirigami.Theme.textColor, 0.12)
    }
}
