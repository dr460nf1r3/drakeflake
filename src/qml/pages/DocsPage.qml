pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../main/helpers.js" as Helpers

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var contents: page.app.docsModel.contents()
    property string currentFile: ""
    property string query: ""
    readonly property var filtered: {
        const searched = page.query.trim().toLowerCase();

        if (searched === "")
            return page.contents.entries;

        return page.contents.entries.filter(entry => entry.title.toLowerCase().includes(searched) || entry.part.toLowerCase().includes(searched));
    }
    // The list has no width during the first layout.
    readonly property real scrollBarLimit: Kirigami.Units.gridUnit * 4
    readonly property real scrollBarWidth: Math.max(0, Math.min(page.scrollBarLimit, page.width - list.x - list.width))
    readonly property real textInset: Kirigami.Units.largeSpacing + Kirigami.Units.smallSpacing * 2

    title: qsTr("Documentation")

    actions: [
        Kirigami.Action {
            text: qsTr("Open Online")
            icon.name: "internet-services"
            displayHint: Kirigami.DisplayHint.IconOnly
            tooltip: qsTr("Open the handbook in the web browser")

            onTriggered: page.app.openExternal("https://dr460nf1r3.github.io/drakeflake/")
        }
    ]
    header: Controls.Control {
        padding: Kirigami.Units.largeSpacing
        topPadding: Kirigami.Units.smallSpacing
        bottomPadding: Kirigami.Units.smallSpacing
        rightPadding: Kirigami.Units.largeSpacing + page.scrollBarWidth

        background: Rectangle {
            color: page.app.vibrancy ? "transparent" : Kirigami.Theme.backgroundColor

            Kirigami.Separator {
                anchors {
                    left: parent.left
                    right: parent.right
                    bottom: parent.bottom
                }
            }
        }
        contentItem: ColumnLayout {
            spacing: Kirigami.Units.smallSpacing

            Kirigami.SearchField {
                id: search

                objectName: "docsSearch"
                focusSequences: []
                Layout.fillWidth: true
                placeholderText: qsTr("Search the documentation…")

                onTextEdited: page.query = search.text
                // The clear button empties the text without textEdited.
                onAccepted: page.query = search.text
            }

            Kirigami.InlineMessage {
                objectName: "docsError"
                visible: page.contents.error !== ""
                Layout.fillWidth: true
                position: Kirigami.InlineMessage.Position.Header
                type: Kirigami.MessageType.Error
                text: Helpers.richText(qsTr("The documentation could not be read: %1").arg(page.contents.error))
            }
        }
    }

    ListView {
        id: list

        objectName: "docsContents"
        model: page.filtered

        delegate: Column {
            id: row

            required property var modelData
            required property int index
            readonly property var before: page.filtered[row.index - 1]
            readonly property bool firstOfPart: row.modelData.part !== "" && (!row.before || row.before.part !== row.modelData.part)

            width: ListView.view.width

            RowLayout {
                id: sectionHeader

                width: search.width + Kirigami.Units.largeSpacing
                visible: row.firstOfPart
                spacing: Kirigami.Units.smallSpacing
                Accessible.role: Accessible.Heading
                Accessible.name: row.modelData.part

                Kirigami.Heading {
                    Layout.alignment: Qt.AlignVCenter
                    Layout.leftMargin: page.textInset
                    Layout.topMargin: row.index === 0 ? Kirigami.Units.smallSpacing : Kirigami.Units.largeSpacing
                    Layout.bottomMargin: Kirigami.Units.smallSpacing
                    Layout.maximumWidth: sectionHeader.width
                    opacity: 0.75
                    level: 5
                    type: Kirigami.Heading.Primary
                    textFormat: Text.PlainText
                    text: row.modelData.part
                    elide: Text.ElideRight
                    font.weight: Font.Bold
                    Accessible.ignored: true
                }

                Kirigami.Separator {
                    Layout.fillWidth: true
                    Layout.alignment: Qt.AlignVCenter
                    Accessible.ignored: true
                }

                // Never grabbed: a flick starting on a header still scrolls.
                TapHandler {
                    grabPermissions: PointerHandler.TakeOverForbidden

                    onTapped: list.forceActiveFocus()
                }
            }

            Controls.ItemDelegate {
                width: row.width
                leftPadding: page.textInset
                text: row.modelData.title
                highlighted: row.modelData.file === page.currentFile

                onClicked: page.app.showDoc(row.modelData.file, "")
            }
        }
    }
}
