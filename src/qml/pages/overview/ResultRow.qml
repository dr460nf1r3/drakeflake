pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

Item {
    id: row

    required property var modelData
    required property int index
    required property bool current
    property real scrollInset: 0
    readonly property bool header: row.modelData.header
    readonly property var group: row.header ? row.modelData.group : null
    readonly property var item: row.header ? null : row.modelData.item

    signal chosen(var target)

    implicitWidth: ListView.view ? ListView.view.width : 0
    implicitHeight: row.header ? heading.implicitHeight : hit.implicitHeight

    Rectangle {
        id: heading

        anchors.fill: parent
        implicitHeight: headingRow.implicitHeight + Kirigami.Units.smallSpacing * 2
        visible: row.header
        color: Kirigami.Theme.alternateBackgroundColor

        RowLayout {
            id: headingRow

            spacing: Kirigami.Units.largeSpacing

            anchors {
                left: parent.left
                right: parent.right
                verticalCenter: parent.verticalCenter
                leftMargin: Kirigami.Units.largeSpacing
                rightMargin: Kirigami.Units.smallSpacing + row.scrollInset
            }

            Controls.Label {
                Layout.fillWidth: true
                text: row.group ? row.group.title : ""
                textFormat: Text.PlainText
                elide: Text.ElideRight
                font.bold: true
                font.pointSize: Kirigami.Theme.smallFont.pointSize
            }

            Controls.Button {
                objectName: "moreButton"
                visible: row.group !== null && row.group.more.length > 0
                text: row.group ? row.group.more : ""
                flat: true
                focusPolicy: Qt.NoFocus

                onClicked: row.chosen(row.group.moreOpen)
            }
        }
    }

    Controls.ItemDelegate {
        id: hit

        objectName: "hit"
        anchors.fill: parent
        visible: !row.header
        focusPolicy: Qt.NoFocus

        background: Rectangle {
            color: Kirigami.ColorUtils.linearInterpolation(Kirigami.Theme.backgroundColor, Kirigami.Theme.textColor, row.current ? 0.1 : hit.hovered ? 0.05 : 0)
        }
        contentItem: RowLayout {
            spacing: Kirigami.Units.largeSpacing

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0

                Controls.Label {
                    Layout.fillWidth: true
                    text: row.item ? row.item.title : ""
                    textFormat: Text.PlainText
                    elide: Text.ElideRight
                    font.family: row.item && row.item.mono ? "monospace" : Kirigami.Theme.defaultFont.family
                    font.weight: Font.DemiBold
                }

                Controls.Label {
                    id: subtitle

                    Layout.fillWidth: true
                    visible: subtitle.text.length > 0
                    text: row.item ? row.item.subtitle : ""
                    textFormat: Text.PlainText
                    elide: Text.ElideRight
                    opacity: 0.75
                    font.pointSize: Kirigami.Theme.smallFont.pointSize
                }
            }

            Controls.Label {
                id: trailing

                readonly property bool flag: trailing.text === "true" || trailing.text === "false"

                Layout.maximumWidth: Kirigami.Units.gridUnit * 12
                visible: trailing.text.length > 0
                text: row.item ? row.item.trailing : ""
                textFormat: Text.PlainText
                elide: Text.ElideRight
                opacity: trailing.flag ? 1 : 0.75
                color: trailing.text === "true" ? Kirigami.Theme.positiveTextColor : trailing.text === "false" ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.textColor
                font.family: row.item && row.item.mono ? "monospace" : Kirigami.Theme.defaultFont.family
                font.weight: trailing.flag ? Font.DemiBold : Font.Normal
                font.pointSize: Kirigami.Theme.smallFont.pointSize
            }

            Item {
                visible: row.scrollInset > 0
                Layout.preferredWidth: row.scrollInset
            }
        }

        onClicked: row.chosen(row.item.open)
    }
}
