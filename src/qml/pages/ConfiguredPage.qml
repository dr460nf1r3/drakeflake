pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../components"
import "../main/helpers.js" as Helpers

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var configured: page.app.configuredModel
    readonly property var changes: page.app.changeModel
    readonly property var system: page.app.systemModel

    function focusSearch(): void {
        configuredSearch.forceActiveFocus();
    }

    title: qsTr("Configured")

    actions: [
        Kirigami.Action {
            text: qsTr("Refresh")
            icon.name: "view-refresh"
            enabled: !page.system.busy
            displayHint: Kirigami.DisplayHint.IconOnly
            tooltip: qsTr("Re-evaluate options and values")

            onTriggered: page.system.reload()
        }
    ]
    header: Controls.Control {
        padding: Kirigami.Units.largeSpacing

        background: SearchHeaderBackground {
            clear: page.app.vibrancy
        }
        contentItem: ColumnLayout {
            spacing: Kirigami.Units.smallSpacing

            DebouncedSearchField {
                id: configuredSearch

                Layout.fillWidth: true
                placeholderText: qsTr("Filter configured options…")
                text: page.app.configuredQuery

                onQueryEdited: query => page.app.configuredQuery = query
                onQueryChanged: query => page.configured.search(query)
            }

            Controls.Label {
                Layout.fillWidth: true
                visible: page.configured.values_ready
                opacity: 0.7
                font: Kirigami.Theme.smallFont
                textFormat: Text.PlainText
                text: qsTr("%1 options set by your configuration on %2").arg(Helpers.count(page.configured.total)).arg(page.system.host)
            }
        }
    }

    ListView {
        id: list

        model: page.configured
        currentIndex: -1
        reuseItems: true

        delegate: Controls.ItemDelegate {
            id: row

            required property int index
            required property string name
            required property string summary
            required property string value
            required property string defined_in
            required property bool read_only
            readonly property string stagedState: {
                page.changes.revision;
                return page.changes.option_state(row.name);
            }

            width: ListView.view.width
            highlighted: false
            Controls.ToolTip.visible: row.hovered && (valueLabel.truncated || row.summary.length > 0)
            Controls.ToolTip.delay: Kirigami.Units.toolTipDelay
            Controls.ToolTip.text: (row.summary.length > 0 ? row.summary + "\n" : "") + valueLabel.text

            background: SelectionBackground {
                selected: row.ListView.isCurrentItem
                hovered: row.hovered
            }
            contentItem: RowLayout {
                spacing: Kirigami.Units.largeSpacing

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0

                    Controls.Label {
                        Layout.fillWidth: true
                        textFormat: Text.PlainText
                        text: row.name
                        font.family: "monospace"
                        font.bold: true
                        wrapMode: Text.WrapAnywhere
                    }

                    Controls.Label {
                        id: valueLabel

                        Layout.fillWidth: true
                        visible: text.length > 0
                        textFormat: Text.PlainText
                        text: row.value.length > 0 ? "= " + row.value : ""
                        font.family: "monospace"
                        color: Kirigami.Theme.linkColor
                        elide: Text.ElideRight
                    }

                    RowLayout {
                        Layout.fillWidth: true
                        spacing: Kirigami.Units.smallSpacing

                        Kirigami.Icon {
                            visible: row.defined_in.length > 0
                            source: "document-edit"
                            Layout.preferredWidth: Kirigami.Units.iconSizes.small
                            Layout.preferredHeight: Kirigami.Units.iconSizes.small
                            opacity: 0.6
                        }

                        Controls.Label {
                            Layout.fillWidth: true
                            visible: row.defined_in.length > 0
                            textFormat: Text.PlainText
                            text: row.defined_in
                            font: Kirigami.Theme.smallFont
                            opacity: 0.7
                            elide: Text.ElideMiddle
                        }

                        Tag {
                            visible: row.stagedState === "managed"
                            text: qsTr("managed")
                            tone: "positive"
                        }

                        Tag {
                            visible: row.stagedState === "set" || row.stagedState === "reset"
                            text: qsTr("staged")
                            tone: "warning"
                        }
                    }
                }

                Kirigami.Icon {
                    source: "go-next"
                    Layout.preferredWidth: Kirigami.Units.iconSizes.small
                    Layout.preferredHeight: Kirigami.Units.iconSizes.small
                    opacity: 0.5
                }
            }

            onClicked: {
                list.currentIndex = row.index;
                page.app.showOption(row.name);
            }
        }
        footer: ShowMoreFooter {
            shown: list.count
            source: page.configured
        }

        Kirigami.PlaceholderMessage {
            anchors.centerIn: parent
            width: parent.width - Kirigami.Units.gridUnit * 4
            visible: list.count === 0
            icon.name: page.configured.values_error.length > 0 ? "data-error" : "view-list-details"
            text: {
                if (page.system.target_error.length > 0)
                    return qsTr("No system selected");
                if (page.configured.values_error.length > 0)
                    return qsTr("Could not evaluate your configuration");
                if (!page.configured.values_ready)
                    return qsTr("Evaluating your configuration…");
                return qsTr("No matching options");
            }
            explanation: {
                if (page.system.target_error.length > 0)
                    return page.system.target_error;
                if (page.configured.values_error.length > 0)
                    return page.configured.values_error;
                if (!page.configured.values_ready)
                    return qsTr("Reading every option your host sets. The first run can take several minutes. Afterwards it is cached.");
                return qsTr("Try another search.");
            }
        }
    }
}
