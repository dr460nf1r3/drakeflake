pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../components"
import "../main/helpers.js" as Helpers
import "functions/names.js" as Names
import "functiondetails"

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var functions: page.app.functionModel
    readonly property bool isFunctionDetails: true
    property string path: ""
    readonly property var info: {
        page.functions.revision;
        return page.functions.details(page.path);
    }
    readonly property var aliases: page.info.aliases || []
    readonly property var args: page.info.args || []

    function openLink(link: string): void {
        const target = Names.linkedFunction(Helpers.unescapeHtml(link));

        if (target.length > 0)
            page.app.showFunction(target);
        else
            page.app.openExternal(link);
    }

    title: page.path

    actions: [
        Kirigami.Action {
            text: qsTr("Copy name")
            icon.name: "edit-copy"

            onTriggered: {
                clipboard.text = page.path;
                clipboard.selectAll();
                clipboard.copy();
            }
        },
        Kirigami.Action {
            text: qsTr("Manual")
            icon.name: "help-contents"
            visible: (page.info.manual || "").length > 0
            tooltip: qsTr("Open this function in the online manual")

            onTriggered: page.app.openExternal(page.info.manual)
        },
        Kirigami.Action {
            text: qsTr("Close")
            icon.name: "dialog-close"
            displayHint: Kirigami.DisplayHint.IconOnly

            onTriggered: page.app.pageStack.removePage(page)
        }
    ]

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        // Copies through a text field: QML has no clipboard API of its own.
        TextEdit {
            id: clipboard

            visible: false
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing

            Kirigami.Icon {
                source: "code-function"
                Layout.preferredWidth: Kirigami.Units.iconSizes.large
                Layout.preferredHeight: Kirigami.Units.iconSizes.large
            }

            Controls.Label {
                Layout.fillWidth: true
                text: page.path
                textFormat: Text.PlainText
                wrapMode: Text.WrapAnywhere

                font {
                    family: "monospace"
                    bold: true
                    pointSize: Kirigami.Theme.defaultFont.pointSize * 1.2
                }
            }
        }

        Kirigami.PlaceholderMessage {
            Layout.fillWidth: true
            visible: page.info.known !== true
            icon.name: "search"
            text: qsTr("No function %1 in the index").arg(page.path)
        }

        Flow {
            Layout.fillWidth: true
            visible: page.info.known === true
            spacing: Kirigami.Units.smallSpacing

            Tag {
                visible: page.info.builtin === true
                text: qsTr("built into Nix")
                tone: "info"
            }

            Tag {
                visible: (page.info.experimental || "").length > 0
                text: qsTr("experimental: %1").arg(page.info.experimental)
                tone: "warning"
            }

            Tag {
                visible: page.info.impure === true
                text: qsTr("impure evaluation only")
                tone: "warning"
            }

            Tag {
                visible: !!page.info.takes
                text: qsTr("takes %1").arg(Names.kindLabel(page.info.takes || "").toLowerCase())
            }

            Tag {
                visible: !!page.info.returns
                text: qsTr("returns %1").arg(Names.kindLabel(page.info.returns || "").toLowerCase())
            }
        }

        CodeBlock {
            objectName: "functionSignature"
            Layout.fillWidth: true
            visible: (page.info.signature || "").length > 0
            text: page.info.signature || ""
            highlightNix: false
        }

        Controls.Label {
            Layout.fillWidth: true
            visible: page.args.length > 0
            text: qsTr("Arguments: %1").arg(page.args.join(", "))
            textFormat: Text.PlainText
            wrapMode: Text.Wrap
            font.family: "monospace"
            opacity: 0.8
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: (page.info.doc_from || "").length > 0
            type: Kirigami.MessageType.Information
            text: Helpers.richText(qsTr("The documentation of %1, which this is an alias of.").arg(page.info.doc_from || ""))
        }

        Controls.Label {
            Layout.fillWidth: true
            visible: page.info.known === true && (page.info.blocks || []).length === 0
            text: qsTr("This function has no documentation comment.")
            opacity: 0.7
        }

        DocBlocks {
            Layout.fillWidth: true
            app: page.app
            blocks: page.info.blocks || []

            onLinkActivated: link => page.openLink(link)
        }

        SectionHeading {
            visible: page.aliases.length > 0
            text: qsTr("Aliases")
        }

        Flow {
            Layout.fillWidth: true
            visible: page.aliases.length > 0
            spacing: Kirigami.Units.smallSpacing

            Repeater {
                model: page.aliases

                delegate: Controls.Button {
                    id: alias

                    required property string modelData

                    text: alias.modelData
                    font.family: "monospace"
                    flat: true
                    icon.name: "code-function"
                    icon.width: Kirigami.Units.iconSizes.sizeForLabels
                    icon.height: Kirigami.Units.iconSizes.sizeForLabels

                    onClicked: page.app.showFunction(alias.modelData)
                }
            }
        }

        SourceSection {
            Layout.fillWidth: true
            app: page.app
            info: page.info
        }
    }
}
