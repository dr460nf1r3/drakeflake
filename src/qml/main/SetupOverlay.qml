pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

import "../pages"

Controls.Popup {
    id: popup

    required property var app
    required property bool shown
    readonly property var page: setupLoader.item

    signal finished

    parent: Controls.Overlay.overlay
    x: 0
    y: 0
    width: popup.parent ? popup.parent.width : 0
    height: popup.parent ? popup.parent.height : 0
    modal: true
    closePolicy: Controls.Popup.NoAutoClose
    padding: 0
    visible: popup.shown

    background: Rectangle {
        color: Kirigami.Theme.backgroundColor
    }
    contentItem: Loader {
        id: setupLoader

        active: popup.shown

        sourceComponent: SetupPage {
            app: popup.app

            onFinished: popup.finished()
        }
    }
}
