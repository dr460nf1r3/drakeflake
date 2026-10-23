pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers
import "names.js" as Names

ColumnLayout {
    id: overview

    required property var app
    readonly property var functions: overview.app.functionModel
    readonly property var cards: overview.functions.namespaces.filter(namespace => namespace.card)
    readonly property var source: overview.functions.source

    signal namespaceChosen(string name)

    spacing: Kirigami.Units.largeSpacing

    SectionHeading {
        text: qsTr("Browse")
    }

    GridLayout {
        id: grid

        Layout.fillWidth: true
        columns: Math.max(1, Math.floor(grid.width / (Kirigami.Units.gridUnit * 11)))
        columnSpacing: Kirigami.Units.largeSpacing
        rowSpacing: Kirigami.Units.largeSpacing

        Repeater {
            model: overview.cards

            delegate: Kirigami.AbstractCard {
                id: card

                required property var modelData
                readonly property string note: Names.namespaceNote(card.modelData.name)

                Layout.fillWidth: true
                Layout.preferredWidth: Kirigami.Units.gridUnit * 10
                showClickFeedback: true
                Accessible.name: card.modelData.name

                contentItem: ColumnLayout {
                    spacing: Kirigami.Units.smallSpacing

                    Controls.Label {
                        Layout.fillWidth: true
                        text: card.modelData.name
                        textFormat: Text.PlainText
                        font.family: "monospace"
                        font.bold: true
                        elide: Text.ElideMiddle
                    }

                    Controls.Label {
                        Layout.fillWidth: true
                        text: card.note.length > 0 ? qsTr("%1 · %2 functions").arg(card.note).arg(Helpers.count(card.modelData.count)) : qsTr("%1 functions").arg(Helpers.count(card.modelData.count))
                        textFormat: Text.PlainText
                        elide: Text.ElideRight
                        color: Kirigami.Theme.disabledTextColor
                    }
                }

                onClicked: overview.namespaceChosen(card.modelData.name)
            }
        }
    }

    Controls.Label {
        Layout.fillWidth: true
        Layout.topMargin: Kirigami.Units.largeSpacing
        visible: overview.source.rev.length > 0
        text: qsTr("%1 functions from nixpkgs %2 (%3) and %4. Built with this app, searched offline.").arg(Helpers.count(overview.functions.size)).arg(Helpers.link(overview.source.url, Helpers.shortRev(overview.source.rev))).arg(Helpers.formatDate(overview.source.date)).arg(Helpers.escapeHtml(overview.source.nix))
        textFormat: Text.StyledText
        wrapMode: Text.Wrap
        font: Kirigami.Theme.smallFont
        opacity: 0.7

        onLinkActivated: link => overview.app.openExternal(link)
    }
}
