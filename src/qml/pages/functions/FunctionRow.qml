import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers

Controls.ItemDelegate {
    id: row

    required property int index
    required property string path
    required property string summary
    required property string signature
    required property bool builtin

    hoverEnabled: true
    down: false
    highlighted: false
    Accessible.name: row.path
    Controls.ToolTip.visible: row.hovered && summaryLabel.truncated
    Controls.ToolTip.delay: Kirigami.Units.toolTipDelay
    Controls.ToolTip.text: Helpers.richText(row.summary)

    background: SelectionBackground {
        selected: row.ListView.isCurrentItem
        hovered: row.hovered
    }
    contentItem: RowLayout {
        spacing: Kirigami.Units.largeSpacing

        Kirigami.Icon {
            source: "code-function"
            Layout.preferredWidth: Kirigami.Units.iconSizes.medium
            Layout.preferredHeight: Kirigami.Units.iconSizes.medium
            Layout.alignment: Qt.AlignTop
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                Controls.Label {
                    Layout.fillWidth: true
                    text: row.path
                    textFormat: Text.PlainText
                    font.bold: true
                    font.family: "monospace"
                    wrapMode: Text.WrapAnywhere
                }

                Tag {
                    visible: row.builtin
                    text: qsTr("builtin")
                    tone: "info"
                }
            }

            Controls.Label {
                Layout.fillWidth: true
                visible: row.signature.length > 0
                text: row.signature
                textFormat: Text.PlainText
                font.family: "monospace"
                font.pointSize: Kirigami.Theme.smallFont.pointSize
                color: Kirigami.Theme.linkColor
                elide: Text.ElideRight
            }

            Controls.Label {
                id: summaryLabel

                Layout.fillWidth: true
                visible: summaryLabel.text.length > 0
                text: row.summary
                textFormat: Text.PlainText
                opacity: 0.75
                elide: Text.ElideRight
                wrapMode: Text.Wrap
                maximumLineCount: 2
            }
        }
    }
}
