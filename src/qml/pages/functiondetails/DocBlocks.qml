pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"

ColumnLayout {
    id: docs

    required property var app
    required property var blocks

    signal linkActivated(string link)

    spacing: Kirigami.Units.largeSpacing

    Repeater {
        model: docs.blocks

        delegate: ColumnLayout {
            id: block

            required property var modelData

            Layout.fillWidth: true
            spacing: 0

            Kirigami.Heading {
                Layout.fillWidth: true
                Layout.topMargin: block.modelData.level <= 1 ? Kirigami.Units.largeSpacing : 0
                visible: block.modelData.kind === "heading"
                level: Math.min(5, (block.modelData.level || 1) + 2)
                text: block.modelData.kind === "heading" ? docs.app.docText(block.modelData.text) : ""
                textFormat: Text.MarkdownText
                wrapMode: Text.Wrap
            }

            Controls.Label {
                id: paragraph

                Layout.fillWidth: true
                visible: block.modelData.kind === "text"
                text: block.modelData.kind === "text" ? docs.app.docText(block.modelData.text) : ""
                textFormat: Text.MarkdownText
                wrapMode: Text.Wrap

                onLinkActivated: link => docs.linkActivated(link)

                HoverHandler {
                    cursorShape: paragraph.hoveredLink ? Qt.PointingHandCursor : Qt.ArrowCursor
                }
            }

            CodeBlock {
                Layout.fillWidth: true
                visible: block.modelData.kind === "code"
                text: block.modelData.kind === "code" ? block.modelData.text : ""
                highlightNix: ["nix", "nix-repl", ""].includes(block.modelData.language || "")
                themeName: docs.app.highlightTheme
            }
        }
    }
}
