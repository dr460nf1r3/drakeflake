import QtQuick
import QtQuick.Controls as Controls

Controls.Label {
    id: label

    textFormat: Text.MarkdownText
    wrapMode: Text.Wrap

    HoverHandler {
        cursorShape: label.hoveredLink ? Qt.PointingHandCursor : Qt.ArrowCursor
    }
}
