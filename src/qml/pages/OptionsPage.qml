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
    readonly property var options: page.app.optionModel
    readonly property var changes: page.app.changeModel
    readonly property var system: page.app.systemModel

    function applyFilter(): void {
        page.options.set_namespace(page.app.optionNamespace);
        page.options.set_only_user(page.app.optionOnlyUser);
        page.options.set_show_templated(page.app.optionShowTemplated);
    }

    function focusSearch(): void {
        search.forceActiveFocus();
    }

    title: qsTr("Options")

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
                id: search

                Layout.fillWidth: true
                menuObjectName: "optionsFieldMenu"
                placeholderText: page.options.size > 0 ? qsTr("Search %1 options…").arg(Helpers.count(page.options.size)) : qsTr("Search options…")
                text: page.app.optionQuery

                onQueryEdited: query => page.app.optionQuery = query
                onQueryChanged: query => page.options.search(query)
            }

            OptionFilters {
                Layout.fillWidth: true
                options: page.options
                area: page.app.optionNamespace
                onlyUser: page.app.optionOnlyUser
                showTemplated: page.app.optionShowTemplated

                onAreaPicked: area => {
                    page.app.optionNamespace = area;
                    page.options.set_namespace(area);
                }
                onOnlyUserToggled: onlyUser => {
                    page.app.optionOnlyUser = onlyUser;
                    page.options.set_only_user(onlyUser);
                }
                onShowTemplatedToggled: showTemplated => {
                    page.app.optionShowTemplated = showTemplated;
                    page.options.set_show_templated(showTemplated);
                }
            }

            Controls.Label {
                Layout.fillWidth: true
                visible: page.options.size > 0
                opacity: 0.7
                font: Kirigami.Theme.smallFont
                textFormat: Text.PlainText
                text: qsTr("%1 matching options").arg(Helpers.count(page.options.total)) + (page.options.values_ready ? "" : qsTr(" · current values are still being evaluated"))
            }
        }
    }

    ListView {
        id: list

        model: page.options
        currentIndex: -1
        reuseItems: true

        delegate: Controls.ItemDelegate {
            id: row

            required property int index
            required property string name
            required property string summary
            required property string type_name
            required property string kind
            required property string value
            required property bool user_set
            required property bool read_only
            readonly property string stagedState: {
                page.changes.revision;
                return page.changes.option_state(row.name);
            }

            width: ListView.view.width
            // A full highlight inverts the text colour. Mark selection subtly instead.
            highlighted: false
            Controls.ToolTip.visible: row.hovered && summaryLabel.truncated
            Controls.ToolTip.delay: Kirigami.Units.toolTipDelay
            Controls.ToolTip.text: Helpers.richText(row.summary)

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
                        id: summaryLabel

                        Layout.fillWidth: true
                        visible: text.length > 0
                        text: row.summary
                        textFormat: Text.PlainText
                        opacity: 0.75
                        elide: Text.ElideRight
                        maximumLineCount: 2
                        wrapMode: Text.Wrap
                    }

                    Flow {
                        Layout.fillWidth: true
                        Layout.topMargin: Kirigami.Units.smallSpacing
                        spacing: Kirigami.Units.smallSpacing

                        Tag {
                            text: row.type_name.length > 40 ? row.kind : row.type_name
                        }

                        Tag {
                            visible: row.value.length > 0
                            text: "= " + row.value
                            tone: "info"
                        }

                        Tag {
                            visible: row.user_set
                            text: qsTr("set in config")
                            tone: "positive"
                        }

                        Tag {
                            visible: row.stagedState === "managed"
                            text: qsTr("managed")
                            tone: "positive"
                        }

                        Tag {
                            visible: row.stagedState === "set" || row.stagedState === "reset"
                            text: row.stagedState === "set" ? qsTr("staged") : qsTr("staged reset")
                            tone: "warning"
                        }

                        Tag {
                            visible: row.read_only
                            text: qsTr("read-only")
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
            source: page.options
        }

        Kirigami.PlaceholderMessage {
            objectName: "optionsPlaceholder"
            anchors.centerIn: parent
            width: parent.width - Kirigami.Units.gridUnit * 4
            visible: list.count === 0
            icon.name: page.options.error.length > 0 || page.system.target_error.length > 0 ? "data-error" : page.options.size === 0 ? "search" : "edit-none"
            text: {
                if (page.system.target_error.length > 0)
                    return qsTr("No system selected");
                if (page.options.error.length > 0 && page.options.size === 0)
                    return qsTr("Could not load NixOS options");
                if (page.options.size === 0)
                    return qsTr("Loading NixOS options…");
                return qsTr("No matching options");
            }
            explanation: {
                if (page.system.target_error.length > 0)
                    return page.system.target_error;
                if (page.options.error.length > 0 && page.options.size === 0)
                    return page.options.error;
                if (page.options.size === 0)
                    return qsTr("The option index is built from your system flake. The first run can take a minute.");
                if (page.app.optionOnlyUser && !page.options.values_ready)
                    return qsTr("Waiting for current values to be evaluated.");
                return qsTr("Try fewer or different words, or another area.");
            }

            helpfulAction: Kirigami.Action {
                enabled: page.system.target_error.length > 0
                text: qsTr("Open Settings")
                icon.name: "settings-configure"

                onTriggered: page.app.showSection("settings")
            }
        }
    }
}
