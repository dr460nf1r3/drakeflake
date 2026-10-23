pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

ColumnLayout {
    id: list

    required property var model
    readonly property var total: list.model.ephemeral_total

    signal persist(string path, string kind)

    spacing: Kirigami.Units.smallSpacing

    Kirigami.Heading {
        level: 3
        text: qsTr("Removed on reboot")
    }

    Controls.Label {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        opacity: 0.75
        font: Kirigami.Theme.smallFont
        textFormat: Text.PlainText
        text: {
            if (list.model.scanning)
                return qsTr("Scanning the root file system…");
            if (!list.model.scanned)
                return qsTr("Not scanned yet.");

            const fileSystem = list.model.file_system || "";
            const where = list.model.compared ? qsTr("%1, compared with the blank snapshot").arg(fileSystem) : fileSystem;
            const size = Qt.locale().formattedDataSize(list.total.size);

            if (list.total.files === 1)
                return qsTr("%1 in 1 file (%2)").arg(size).arg(where);

            return qsTr("%1 in %2 files (%3)").arg(size).arg(list.total.files.toLocaleString(Qt.locale(), "f", 0)).arg(where);
        }
    }

    ListCard {
        id: card

        Layout.fillWidth: true
        Layout.fillHeight: true
        listName: "ephemeralList"
        model: list.model.ephemeral
        hintShown: card.count === 0 && !list.model.scanning && list.total.files === 0
        hint: list.model.scanned ? qsTr("Nothing is removed on reboot") : qsTr("Scan to see what a reboot removes")
        hintExplanation: list.model.scanned ? "" : qsTr("Click Scan above. It needs the administrator password to read every folder.")

        delegate: Controls.ItemDelegate {
            id: row

            required property var modelData

            width: ListView.view.width
            hoverEnabled: true
            leftPadding: Kirigami.Units.largeSpacing + row.modelData.depth * Kirigami.Units.gridUnit

            contentItem: RowLayout {
                spacing: Kirigami.Units.smallSpacing

                Kirigami.Icon {
                    source: row.modelData.expandable ? (row.modelData.expanded ? "go-down" : "go-next") : ""
                    Layout.preferredWidth: Kirigami.Units.iconSizes.small
                    Layout.preferredHeight: Kirigami.Units.iconSizes.small
                }

                KindIcon {
                    kind: row.modelData.kind
                }

                Controls.Label {
                    Layout.fillWidth: true
                    elide: Text.ElideMiddle
                    textFormat: Text.PlainText
                    text: row.modelData.name
                }

                Controls.Label {
                    opacity: 0.7
                    font: Kirigami.Theme.smallFont
                    textFormat: Text.PlainText
                    text: {
                        if (row.modelData.denied)
                            return qsTr("not readable");

                        const size = Qt.locale().formattedDataSize(row.modelData.size);

                        if (row.modelData.kind !== "dir")
                            return size;

                        return row.modelData.files === 1 ? qsTr("%1, 1 file").arg(size) : qsTr("%1, %2 files").arg(size).arg(row.modelData.files);
                    }
                }

                RowButton {
                    objectName: "persist"
                    icon.name: "list-add"
                    text: qsTr("Keep")
                    hint: row.modelData.kind === "dir" ? qsTr("Keep this folder across reboots") : qsTr("Keep this file across reboots")

                    onClicked: list.persist(row.modelData.path, row.modelData.kind)
                }
            }

            onClicked: {
                if (row.modelData.expandable)
                    list.model.toggle(row.modelData.path);
            }
        }
    }
}
