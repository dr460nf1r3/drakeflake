import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

Item {
    id: footer

    required property var source
    required property int shown

    width: ListView.view ? ListView.view.width : 0
    height: footer.source.has_more ? more.implicitHeight + Kirigami.Units.largeSpacing * 2 : 0

    Controls.Button {
        id: more

        anchors.centerIn: parent
        visible: footer.source.has_more
        text: qsTr("Show more (%1 left)").arg(footer.source.total - footer.shown)
        icon.name: "go-down"
        icon.width: Kirigami.Units.iconSizes.sizeForLabels
        icon.height: Kirigami.Units.iconSizes.sizeForLabels

        onClicked: footer.source.load_more()
    }
}
