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
    readonly property var configured: page.app.homeModel
    readonly property var userList: page.system.home_users
    readonly property string prefix: "home-manager.users." + page.system.home_user + "."
    readonly property var changes: page.app.changeModel
    readonly property var system: page.app.systemModel
    readonly property string homeState: page.system.home_state

    function applyFilter(): void {
        page.configured.set_namespace(page.app.homeNamespace);
        page.configured.set_only_user(page.app.homeOnlyUser);
        page.configured.set_show_templated(page.app.homeShowTemplated);
    }

    function focusSearch(): void {
        configuredSearch.forceActiveFocus();
    }

    title: qsTr("Home Manager")

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
        visible: page.homeState === "available"
        padding: Kirigami.Units.largeSpacing

        background: SearchHeaderBackground {
            clear: page.app.vibrancy
        }
        contentItem: ColumnLayout {
            spacing: Kirigami.Units.smallSpacing

            DebouncedSearchField {
                id: configuredSearch

                Layout.fillWidth: true
                placeholderText: page.configured.size > 0 ? qsTr("Search %1 Home Manager options of %2…").arg(Helpers.count(page.configured.size)).arg(page.system.home_user) : qsTr("Search Home Manager options…")
                text: page.app.homeQuery

                onQueryEdited: query => page.app.homeQuery = query
                onQueryChanged: query => page.configured.search(query)
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                FilterPicker {
                    objectName: "userPicker"
                    visible: page.userList.length > 1
                    options: page.userList.map(name => ({
                                value: name,
                                label: name
                            }))
                    currentValue: page.system.home_user
                    filterPlaceholder: qsTr("Filter users…")

                    onPicked: value => page.system.select_home_user(value)
                }

                OptionFilters {
                    Layout.fillWidth: true
                    options: page.configured
                    area: page.app.homeNamespace
                    onlyUser: page.app.homeOnlyUser
                    showTemplated: page.app.homeShowTemplated

                    onAreaPicked: area => {
                        page.app.homeNamespace = area;
                        page.configured.set_namespace(area);
                    }
                    onOnlyUserToggled: onlyUser => {
                        page.app.homeOnlyUser = onlyUser;
                        page.configured.set_only_user(onlyUser);
                    }
                    onShowTemplatedToggled: showTemplated => {
                        page.app.homeShowTemplated = showTemplated;
                        page.configured.set_show_templated(showTemplated);
                    }
                }
            }

            Controls.Label {
                Layout.fillWidth: true
                visible: page.configured.values_ready
                opacity: 0.7
                font: Kirigami.Theme.smallFont
                textFormat: Text.PlainText
                text: qsTr("%1 matching options").arg(Helpers.count(page.configured.total))
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
                        text: row.name.startsWith(page.prefix) ? row.name.slice(page.prefix.length) : row.name
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
                page.app.showOption(row.name, page.configured);
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
            icon.name: page.configured.values_error.length > 0 || page.homeState.startsWith("error") ? "data-error" : page.homeState === "none" ? "user" : "view-list-details"
            text: {
                if (page.system.target_error.length > 0)
                    return qsTr("No system selected");
                if (page.homeState === "")
                    return qsTr("Checking for Home Manager…");
                if (page.homeState === "none")
                    return qsTr("Home Manager is not set up on this host");
                if (page.homeState.startsWith("error"))
                    return qsTr("Could not check for Home Manager");
                if (page.configured.values_error.length > 0)
                    return qsTr("Could not evaluate your configuration");
                if (!page.configured.values_ready)
                    return qsTr("Reading Home Manager options…");
                return qsTr("No matching options");
            }
            explanation: {
                if (page.system.target_error.length > 0)
                    return page.system.target_error;
                if (page.homeState === "")
                    return qsTr("The host's configuration is being read.");
                if (page.homeState === "none")
                    return qsTr("Import Home Manager as a system module in your flake (home-manager.nixosModules.home-manager, or darwinModules on a Mac) and set up a user under home-manager.users. Their settings then show here.");
                if (page.homeState.startsWith("error"))
                    return page.homeState.slice("error: ".length);
                if (page.configured.values_error.length > 0)
                    return page.configured.values_error;
                if (!page.configured.values_ready)
                    return qsTr("Home Manager options and values of %1 are being evaluated.").arg(page.system.home_user);
                return qsTr("Try another search.");
            }
        }
    }
}
