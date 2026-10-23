pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"

ColumnLayout {
    id: guide

    required property var app
    required property var blocks

    signal linkActivated(string link)

    function headingItem(anchor: string): Item {
        for (let i = 0; i < repeater.count; i++) {
            const block = guide.blocks[i];

            if (block.kind === "heading" && block.anchor === anchor)
                return repeater.itemAt(i);
        }

        return null;
    }

    spacing: Kirigami.Units.largeSpacing

    Repeater {
        id: repeater

        model: guide.blocks

        delegate: Loader {
            id: block

            required property var modelData

            Layout.fillWidth: true
            Layout.topMargin: block.modelData.kind === "heading" ? Kirigami.Units.largeSpacing : 0
            objectName: "guide-" + block.modelData.kind
            sourceComponent: {
                switch (block.modelData.kind) {
                case "heading":
                    return heading;
                case "code":
                    return code;
                case "image":
                    return image;
                case "note":
                    return note;
                default:
                    return text;
                }
            }

            Component {
                id: heading

                Kirigami.Heading {
                    level: Math.min(5, block.modelData.level)
                    text: guide.app.guideText(block.modelData.text)
                    textFormat: Text.MarkdownText
                    wrapMode: Text.Wrap
                }
            }

            Component {
                id: text

                GuideText {
                    text: guide.app.guideText(block.modelData.text)

                    onLinkActivated: link => guide.linkActivated(link)
                }
            }

            Component {
                id: code

                CodeBlock {
                    text: block.modelData.text
                    highlightNix: block.modelData.language === "nix"
                    themeName: guide.app.highlightTheme
                }
            }

            Component {
                id: image

                GuideImage {
                    source: block.modelData.source
                    alt: block.modelData.alt
                    preferredWidth: block.modelData.width
                    naturalWidth: block.modelData.natural_width
                    naturalHeight: block.modelData.natural_height
                }
            }

            Component {
                id: note

                GuideNote {
                    warning: block.modelData.tone === "warning"
                    text: guide.app.guideText(block.modelData.text)

                    onLinkActivated: link => guide.linkActivated(link)
                }
            }
        }
    }
}
