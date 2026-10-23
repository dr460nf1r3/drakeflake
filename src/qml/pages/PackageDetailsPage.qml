import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../components"
import "../main/helpers.js" as Helpers

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var packages: page.app.packageModel
    readonly property var changes: page.app.changeModel
    readonly property bool isPackageDetails: true
    property string attr: ""
    property string stageError: ""
    property bool trying: false
    readonly property var info: {
        page.packages.revision;

        return page.packages.details(page.attr);
    }
    readonly property var meta: page.info.meta || {}
    readonly property string pkgState: {
        page.changes.revision;
        return page.changes.package_state(page.attr);
    }

    function stage(add: bool): void {
        page.stageError = page.changes.stage_package(page.attr, add);
    }

    function requestDetails(): void {
        if (page.packages && page.attr.length > 0)
            page.packages.request_details(page.attr);
    }

    title: page.attr

    actions: [
        Kirigami.Action {
            text: qsTr("Homepage")
            icon.name: "internet-services"
            visible: !!page.meta.homepage

            onTriggered: page.app.openExternal(page.meta.homepage)
        },
        Kirigami.Action {
            text: qsTr("Close")
            icon.name: "dialog-close"
            displayHint: Kirigami.DisplayHint.IconOnly

            onTriggered: page.app.pageStack.removePage(page)
        }
    ]

    // Deferred: `attr` is assigned while the page is still being created.
    onAttrChanged: {
        page.stageError = "";
        page.trying = false;
        Qt.callLater(page.requestDetails);
    }
    Component.onCompleted: Qt.callLater(page.requestDetails)

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing

            Kirigami.Icon {
                source: "package-x-generic"
                Layout.preferredWidth: Kirigami.Units.iconSizes.large
                Layout.preferredHeight: Kirigami.Units.iconSizes.large
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0

                Controls.Label {
                    Layout.fillWidth: true
                    text: page.attr
                    wrapMode: Text.WrapAnywhere

                    font {
                        family: "monospace"
                        bold: true
                        pointSize: Kirigami.Theme.defaultFont.pointSize * 1.2
                    }
                }

                Controls.Label {
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: page.meta.version || (page.info.summary ? page.info.summary.version : "")
                    textFormat: Text.PlainText
                    opacity: 0.7
                }
            }
        }

        Flow {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            Tag {
                visible: page.info.installed === true
                text: qsTr("installed")
                tone: "positive"
            }

            Tag {
                visible: page.pkgState === "managed"
                text: qsTr("added by DrakeFlake")
                tone: "positive"
            }

            Tag {
                visible: page.pkgState === "add"
                text: qsTr("will be added")
                tone: "warning"
            }

            Tag {
                visible: page.pkgState === "remove"
                text: qsTr("will be removed")
                tone: "negative"
            }

            Tag {
                visible: page.meta.unfree === true
                text: qsTr("unfree")
                tone: "warning"
            }

            Tag {
                visible: page.meta.broken === true
                text: qsTr("marked broken")
                tone: "negative"
            }
        }

        Controls.Label {
            Layout.fillWidth: true
            text: page.meta.description || (page.info.summary ? page.info.summary.description : "")
            textFormat: Text.PlainText
            wrapMode: Text.Wrap
        }

        Flow {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            Controls.Button {
                visible: page.pkgState === "" && page.info.installed !== true
                text: qsTr("Add to system")
                icon.name: "list-add"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels
                highlighted: true

                onClicked: page.stage(true)
            }

            Controls.Button {
                visible: page.pkgState === "managed"
                text: qsTr("Remove from system")
                icon.name: "list-remove"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels

                onClicked: page.stage(false)
            }

            Controls.Button {
                visible: page.pkgState === "add" || page.pkgState === "remove"
                text: qsTr("Undo")
                icon.name: "edit-undo"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels

                onClicked: page.changes.unstage("package:" + page.attr)
            }

            Controls.Button {
                id: tryButton

                visible: !!page.meta.mainProgram
                text: qsTr("Try it")
                icon.name: "media-playback-start"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels
                Controls.ToolTip.visible: tryButton.hovered
                Controls.ToolTip.text: qsTr("Run it without installing: apps open as a window, command-line tools in a terminal with the package available")

                onClicked: {
                    page.stageError = page.app.packageModel.try_package(page.attr);
                    page.trying = page.stageError.length === 0;
                }
            }

            Controls.Label {
                visible: page.pkgState === "" && page.info.installed === true
                text: qsTr("Installed by your configuration")
                opacity: 0.7
            }
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: page.trying
            type: Kirigami.MessageType.Information
            text: qsTr("Fetching %1 from your system's nixpkgs to try it. It starts by itself when ready. Progress and the result are in the Console.").arg(page.attr)
            showCloseButton: true
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: page.attr.indexOf(".") >= 0 && page.pkgState === ""
            type: Kirigami.MessageType.Information
            text: qsTr("This comes from the package set %1. Libraries from sets usually go into an environment (for example python3.withPackages) rather than directly into the system packages.").arg(page.attr.split(".")[0])
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: page.stageError.length > 0
            type: Kirigami.MessageType.Error
            text: page.stageError
        }

        RowLayout {
            Layout.fillWidth: true
            visible: page.info.state === "loading" || page.info.state === "unknown"

            Controls.BusyIndicator {
                running: parent.visible
                Layout.preferredWidth: Kirigami.Units.iconSizes.sizeForLabels
                Layout.preferredHeight: Kirigami.Units.iconSizes.sizeForLabels
            }

            Controls.Label {
                text: qsTr("Reading package details…")
                opacity: 0.7
            }
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: page.info.state === "error"
            type: Kirigami.MessageType.Warning
            text: Helpers.richText(page.info.error || "")
        }

        SectionHeading {
            visible: !!page.meta.longDescription
            text: qsTr("About")
        }

        Controls.Label {
            Layout.fillWidth: true
            visible: !!page.meta.longDescription
            text: page.app.docText(page.meta.longDescription || "")
            textFormat: Text.MarkdownText
            wrapMode: Text.Wrap

            onLinkActivated: link => page.app.openExternal(link)
        }

        // Left-aligned label/value grid (FormLayout centres itself on wide pages).
        GridLayout {
            Layout.fillWidth: true
            visible: page.info.state === "ready"
            columns: 2
            columnSpacing: Kirigami.Units.largeSpacing
            rowSpacing: Kirigami.Units.smallSpacing

            FieldLabel {
                visible: program.visible
                text: qsTr("Program:")
            }

            Controls.Label {
                id: program

                Layout.fillWidth: true
                visible: !!page.meta.mainProgram
                text: page.meta.mainProgram || ""
                textFormat: Text.PlainText
                font.family: "monospace"
                wrapMode: Text.WrapAnywhere
            }

            FieldLabel {
                visible: license.visible
                text: qsTr("License:")
            }

            Controls.Label {
                id: license

                Layout.fillWidth: true
                visible: (page.meta.licenses || []).length > 0
                text: (page.meta.licenses || []).join(", ")
                textFormat: Text.PlainText
                wrapMode: Text.Wrap
            }

            FieldLabel {
                visible: homepage.visible
                text: qsTr("Homepage:")
            }

            Controls.Label {
                id: homepage

                Layout.fillWidth: true
                visible: !!page.meta.homepage
                text: Helpers.link(page.meta.homepage || "", page.meta.homepage || "")
                textFormat: Text.StyledText
                wrapMode: Text.WrapAnywhere

                onLinkActivated: link => page.app.openExternal(link)
            }

            FieldLabel {
                visible: maintainers.visible
                text: qsTr("Maintainers:")
            }

            Controls.Label {
                id: maintainers

                Layout.fillWidth: true
                visible: (page.meta.maintainers || []).length > 0
                text: (page.meta.maintainers || []).filter(m => !!m).join(", ")
                textFormat: Text.PlainText
                wrapMode: Text.Wrap
            }

            FieldLabel {
                visible: outputs.visible
                text: qsTr("Outputs:")
            }

            Controls.Label {
                id: outputs

                Layout.fillWidth: true
                visible: (page.meta.outputs || []).length > 1
                text: (page.meta.outputs || []).join(", ")
                textFormat: Text.PlainText
                font.family: "monospace"
                wrapMode: Text.Wrap
            }

            FieldLabel {
                visible: definedIn.visible
                text: qsTr("Defined in:")
            }

            Controls.Label {
                id: definedIn

                readonly property string position: (page.meta.position || "").replace(/:(\d+)$/, "#L$1")

                Layout.fillWidth: true
                visible: definedIn.position.length > 0
                text: Helpers.link("https://github.com/NixOS/nixpkgs/blob/master/" + definedIn.position, page.meta.position || "")
                textFormat: Text.StyledText
                wrapMode: Text.WrapAnywhere
                font.family: "monospace"

                onLinkActivated: link => page.app.openExternal(link)
            }
        }

        Item {
            Layout.fillHeight: true
        }
    }
}
