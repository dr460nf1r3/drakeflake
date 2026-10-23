import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers

Controls.ItemDelegate {
    id: row

    required property int index
    required property string attr
    required property string version
    required property string description
    required property bool installed
    required property var app
    required property var changes
    required property string packageSet
    readonly property string pkgState: {
        row.changes.revision;
        return row.changes.package_state(row.attr);
    }

    signal stageRequested(bool add)
    signal undoRequested

    hoverEnabled: true
    down: false
    highlighted: false
    Controls.ToolTip.visible: row.hovered && descriptionLabel.truncated
    Controls.ToolTip.delay: Kirigami.Units.toolTipDelay
    Controls.ToolTip.text: Helpers.richText(row.description)

    background: SelectionBackground {
        selected: row.ListView.isCurrentItem
        hovered: row.hovered
    }
    contentItem: RowLayout {
        spacing: Kirigami.Units.largeSpacing

        Kirigami.Icon {
            source: "package-x-generic"
            Layout.preferredWidth: Kirigami.Units.iconSizes.medium
            Layout.preferredHeight: Kirigami.Units.iconSizes.medium
            opacity: row.installed || row.pkgState === "managed" ? 1 : 0.5
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            RowLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                Controls.Label {
                    objectName: "attrLabel"
                    text: {
                        const prefix = row.packageSet + ".";
                        return row.packageSet.length > 0 && row.attr.startsWith(prefix) ? row.attr.slice(prefix.length) : row.attr;
                    }
                    textFormat: Text.PlainText
                    font.bold: true
                    font.family: "monospace"
                    wrapMode: Text.WrapAnywhere
                    Layout.maximumWidth: row.width * 0.6
                }

                Controls.Label {
                    visible: row.version.length > 0
                    textFormat: Text.PlainText
                    text: row.version
                    opacity: 0.6
                }

                Item {
                    Layout.fillWidth: true
                }
            }

            Controls.Label {
                id: descriptionLabel

                objectName: "descriptionLabel"
                Layout.fillWidth: true
                visible: descriptionLabel.text.length > 0
                text: row.description
                textFormat: Text.PlainText
                opacity: 0.75
                elide: Text.ElideRight
                wrapMode: Text.Wrap
                maximumLineCount: 2
            }

            Flow {
                id: tags

                Layout.fillWidth: true
                Layout.topMargin: tags.visible ? Kirigami.Units.smallSpacing : 0
                visible: row.installed || row.pkgState !== ""
                spacing: Kirigami.Units.smallSpacing

                Tag {
                    visible: row.installed
                    text: qsTr("installed")
                    tone: "positive"
                }

                Tag {
                    visible: row.pkgState === "managed"
                    text: qsTr("added by DrakeFlake")
                    tone: "positive"
                }

                Tag {
                    visible: row.pkgState === "add"
                    text: qsTr("will be added")
                    tone: "warning"
                }

                Tag {
                    visible: row.pkgState === "remove"
                    text: qsTr("will be removed")
                    tone: "negative"
                }
            }
        }

        Controls.Button {
            objectName: "undoButton"
            visible: row.pkgState === "add" || row.pkgState === "remove"
            text: qsTr("Undo")
            icon.name: "edit-undo"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels

            onClicked: row.undoRequested()
        }

        Controls.Button {
            objectName: "removeButton"
            visible: row.pkgState === "managed"
            text: qsTr("Remove")
            icon.name: "list-remove"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels

            onClicked: row.stageRequested(false)
        }

        Controls.Button {
            objectName: "addButton"
            visible: row.pkgState === "" && !row.installed
            text: qsTr("Add")
            icon.name: "list-add"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels

            onClicked: row.stageRequested(true)
        }

        Controls.Label {
            visible: row.pkgState === "" && row.installed
            text: qsTr("From your config")
            opacity: 0.6
            font: Kirigami.Theme.smallFont
        }
    }
}
