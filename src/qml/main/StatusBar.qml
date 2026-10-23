import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Item {
    id: bar

    required property bool busy
    required property bool running
    required property string activity
    required property bool consoleShown
    required property real contentX
    required property real windowWidth
    property bool clear: false
    property bool reduceMotion: false

    signal showConsole

    visible: bar.busy || bar.running
    height: visible ? statusRow.implicitHeight + Kirigami.Units.smallSpacing * 2 : 0

    Rectangle {
        x: bar.contentX
        width: bar.windowWidth
        height: parent.height
        color: bar.clear ? "transparent" : Kirigami.Theme.backgroundColor

        Kirigami.Separator {
            anchors {
                left: parent.left
                right: parent.right
                top: parent.top
            }
        }

        RowLayout {
            id: statusRow

            spacing: Kirigami.Units.largeSpacing

            anchors {
                fill: parent
                leftMargin: Kirigami.Units.largeSpacing
                rightMargin: Kirigami.Units.largeSpacing
            }

            Kirigami.Icon {
                source: "view-refresh"
                Layout.preferredWidth: Kirigami.Units.iconSizes.small
                Layout.preferredHeight: Kirigami.Units.iconSizes.small

                RotationAnimator on rotation {
                    from: 0
                    to: 360
                    duration: 1500
                    loops: Animation.Infinite
                    running: statusRow.visible && !bar.reduceMotion
                }
            }

            Controls.Label {
                Layout.fillWidth: true
                elide: Text.ElideRight
                textFormat: Text.PlainText
                text: bar.running ? qsTr("Applying changes…") : bar.activity.split("\n").join("  ·  ")
            }

            Controls.Button {
                visible: bar.running && !bar.consoleShown
                text: qsTr("Show console")
                icon.name: "dialog-scripts"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels
                flat: true

                onClicked: bar.showConsole()
            }
        }
    }
}
