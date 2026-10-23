import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Controls.ToolBar {
    id: footer

    required property int step
    required property bool canContinue
    required property bool canUseFlake
    required property bool imported

    signal back
    signal useFlake
    signal next
    signal finish

    contentItem: Item {
        implicitHeight: Math.max(backRow.implicitHeight, nextRow.implicitHeight)

        RowLayout {
            id: backRow

            anchors.left: parent.left
            anchors.verticalCenter: parent.verticalCenter

            Controls.Button {
                visible: footer.step > 0
                text: qsTr("Back")
                icon.name: "go-previous"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels

                onClicked: footer.back()
            }
        }

        RowLayout {
            id: nextRow

            anchors.right: parent.right
            anchors.verticalCenter: parent.verticalCenter

            Controls.Button {
                visible: footer.step === 2 && !footer.imported
                text: qsTr("Finish later")
                flat: true

                onClicked: footer.finish()
            }

            Controls.Button {
                visible: footer.step === 1 && footer.canUseFlake
                text: qsTr("Use this flake")
                icon.name: "dialog-ok-apply"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels
                highlighted: true

                onClicked: footer.useFlake()
            }

            Controls.Button {
                visible: footer.step < 2
                text: qsTr("Next")
                icon.name: "go-next"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels
                highlighted: footer.step === 0 || footer.canContinue
                enabled: footer.step === 0 || footer.canContinue

                onClicked: footer.next()
            }

            Controls.Button {
                visible: footer.step === 2
                text: qsTr("Finish")
                icon.name: "dialog-ok-apply"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels
                highlighted: true
                enabled: footer.imported

                onClicked: footer.finish()
            }
        }
    }
}
