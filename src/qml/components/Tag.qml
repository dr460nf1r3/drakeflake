import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

import "../main/helpers.js" as Helpers

Rectangle {
    id: tag

    property alias text: label.text
    property string tone: "neutral"
    readonly property color toneColor: {
        switch (tag.tone) {
        case "positive":
            return Kirigami.Theme.positiveTextColor;
        case "warning":
            return Kirigami.Theme.neutralTextColor;
        case "negative":
            return Kirigami.Theme.negativeTextColor;
        case "info":
            return Kirigami.Theme.linkColor;
        default:
            return Kirigami.Theme.disabledTextColor;
        }
    }

    implicitWidth: label.implicitWidth + Kirigami.Units.largeSpacing * 1.5
    width: parent ? Math.min(implicitWidth, parent.width) : implicitWidth
    implicitHeight: label.implicitHeight + Kirigami.Units.smallSpacing
    radius: height / 2
    color: Qt.alpha(toneColor, 0.15)
    border.width: 1
    border.color: Qt.alpha(toneColor, 0.5)
    Controls.ToolTip.visible: hover.hovered && label.truncated
    Controls.ToolTip.delay: Kirigami.Units.toolTipDelay
    Controls.ToolTip.text: Helpers.richText(label.text)

    HoverHandler {
        id: hover

        objectName: "hover"
    }

    Controls.Label {
        id: label

        horizontalAlignment: Text.AlignHCenter
        color: tag.tone === "neutral" ? Kirigami.Theme.textColor : tag.toneColor
        font: Kirigami.Theme.smallFont
        elide: Text.ElideRight
        textFormat: Text.PlainText

        anchors {
            left: parent.left
            right: parent.right
            verticalCenter: parent.verticalCenter
            leftMargin: Kirigami.Units.largeSpacing * 0.75
            rightMargin: Kirigami.Units.largeSpacing * 0.75
        }
    }
}
