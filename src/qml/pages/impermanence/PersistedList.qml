pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"

ColumnLayout {
    id: list

    required property var model

    signal unpersist(string option, string entry)
    signal keep(string option, string entry)

    spacing: Kirigami.Units.smallSpacing

    Kirigami.Heading {
        level: 3
        text: qsTr("Persisted (%1)").arg(card.count)
    }

    Controls.Label {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        opacity: 0.75
        font: Kirigami.Theme.smallFont
        text: list.model.scope === "home" ? qsTr("Folders and files of this user that survive a reboot.") : qsTr("Folders and files of the system that survive a reboot.")
    }

    ListCard {
        id: card

        Layout.fillWidth: true
        Layout.fillHeight: true
        listName: "persistedList"
        model: list.model.persisted
        hint: qsTr("Nothing is persisted")
        hintExplanation: qsTr("Everything on the root file system is removed on reboot.")

        delegate: Controls.ItemDelegate {
            id: row

            required property var modelData

            width: ListView.view.width
            hoverEnabled: true

            contentItem: RowLayout {
                spacing: Kirigami.Units.smallSpacing

                KindIcon {
                    kind: row.modelData.kind
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0

                    Controls.Label {
                        Layout.fillWidth: true
                        elide: Text.ElideMiddle
                        textFormat: Text.PlainText
                        font.strikeout: row.modelData.state === "remove"
                        text: row.modelData.path
                    }

                    Controls.Label {
                        Layout.fillWidth: true
                        elide: Text.ElideRight
                        opacity: 0.7
                        font: Kirigami.Theme.smallFont
                        textFormat: Text.PlainText
                        text: {
                            if (row.modelData.locked)
                                return row.modelData.source.length > 0 ? qsTr("in %1, set in %2").arg(row.modelData.storage).arg(row.modelData.source) : qsTr("in %1, set in your configuration").arg(row.modelData.storage);
                            return qsTr("in %1, set by DrakeFlake").arg(row.modelData.storage);
                        }
                    }
                }

                Tag {
                    visible: row.modelData.state !== ""
                    tone: row.modelData.state === "add" ? "positive" : "negative"
                    text: row.modelData.state === "add" ? qsTr("staged") : qsTr("staged removal")
                }

                RowButton {
                    objectName: "unpersist"
                    visible: row.modelData.state !== "remove"
                    enabled: !row.modelData.locked
                    icon.name: "list-remove"
                    text: qsTr("Do not keep")
                    hint: row.modelData.locked ? qsTr("Set outside the generated file. Remove it in the file that sets it.") : qsTr("Removed from the root file system on the next reboot")

                    onClicked: list.unpersist(row.modelData.option, row.modelData.entry)
                }

                RowButton {
                    objectName: "keep"
                    visible: row.modelData.state === "remove"
                    icon.name: "edit-undo"
                    text: qsTr("Keep it")

                    onClicked: list.keep(row.modelData.option, row.modelData.entry)
                }
            }
        }
    }
}
