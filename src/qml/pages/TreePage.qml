pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../components"
import "../main/helpers.js" as Helpers
import "tree"

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var system: page.app.systemModel
    readonly property bool hasHome: page.app.homeModel.size > 0 || page.system.home_users.length > 0
    property string scope: "system"
    property string query: ""
    readonly property var options: page.scope === "home" ? page.app.homeModel : page.app.optionModel
    readonly property var treeModel: page.scope === "home" ? page.app.homeTreeModel : page.app.treeModel
    readonly property string homePrefix: "home-manager.users." + page.system.home_user + "."
    readonly property var found: {
        if (page.query === "")
            return null;
        page.options.revision;
        return page.options.top(page.query, 30);
    }
    readonly property bool allOpen: page.treeModel.all_open
    property real keptContentY: 0

    function reset(): void {
        page.query = "";
        search.text = "";
    }

    function focusSearch(): void {
        search.forceActiveFocus();
    }

    title: qsTr("Tree View")

    // Page actions cannot show Collapse all as a toggle, so it lives in the header.
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
                focusOnCreate: false
                placeholderText: qsTr("Search options…")

                onQueryEdited: query => page.query = query
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                Controls.ComboBox {
                    Layout.fillWidth: true
                    visible: page.hasHome
                    model: [qsTr("System options"), qsTr("Home Manager options")]
                    currentIndex: page.scope === "home" ? 1 : 0

                    onActivated: index => {
                        page.scope = index === 1 ? "home" : "system";
                        page.reset();
                    }
                }

                IconButton {
                    text: page.allOpen ? qsTr("Collapse all") : qsTr("Expand all")
                    icon.name: page.allOpen ? "go-up" : "go-down"

                    onClicked: {
                        if (page.allOpen)
                            page.treeModel.collapse_all();
                        else
                            page.treeModel.expand_all();
                    }
                }
            }

            Controls.Label {
                Layout.fillWidth: true
                visible: page.options.size > 0 && !page.options.values_ready
                opacity: 0.7
                font: Kirigami.Theme.smallFont
                textFormat: Text.PlainText
                text: qsTr("Current values are still being evaluated — defaults are shown meanwhile")
            }
        }
    }

    onTreeModelChanged: page.treeModel.open_tree()
    Component.onCompleted: page.treeModel.open_tree()

    ColumnLayout {
        id: waiting

        readonly property bool indexMissing: page.options.size === 0

        anchors.centerIn: parent
        width: parent.width - Kirigami.Units.gridUnit * 4
        visible: (waiting.indexMissing || page.treeModel.tree_building && tree.count === 0) && page.system.target_error.length === 0
        spacing: Kirigami.Units.largeSpacing

        Controls.BusyIndicator {
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: Kirigami.Units.iconSizes.smallMedium
            Layout.preferredHeight: Kirigami.Units.iconSizes.smallMedium
            running: visible
        }

        Controls.Label {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            textFormat: Text.PlainText
            text: waiting.indexMissing ? qsTr("Loading options…") : qsTr("Building the tree…")
        }

        Controls.Label {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            visible: waiting.indexMissing
            opacity: 0.7
            font: Kirigami.Theme.smallFont
            textFormat: Text.PlainText
            wrapMode: Text.Wrap
            text: qsTr("The option index is built from your system flake. The first run can take a minute.")
        }
    }

    Connections {
        function onModelAboutToBeReset(): void {
            page.keptContentY = tree.contentY;
        }

        function onModelReset(): void {
            tree.contentY = Math.min(page.keptContentY, Math.max(0, tree.contentHeight - tree.height));
        }

        target: page.treeModel
    }

    ListView {
        id: tree

        objectName: "optionTree"
        visible: page.query === "" && page.options.size > 0
        model: page.treeModel
        currentIndex: -1
        reuseItems: true

        delegate: Controls.ItemDelegate {
            id: node

            required property int index
            required property string name
            required property string label
            required property bool is_option
            required property bool has_children
            required property int option_count
            required property string value
            required property bool user_set
            required property string summary
            required property int depth
            required property bool open

            width: ListView.view.width
            highlighted: false
            Controls.ToolTip.visible: node.hovered && node.summary.length > 0
            Controls.ToolTip.delay: Kirigami.Units.toolTipDelay
            Controls.ToolTip.text: Helpers.richText(node.summary)

            background: SelectionBackground {
                selected: node.ListView.isCurrentItem
                hovered: node.hovered
            }
            contentItem: RowLayout {
                spacing: Kirigami.Units.smallSpacing

                // Bound to the depth so a reused row never keeps the width it had before.
                Item {
                    Layout.preferredWidth: node.depth * Kirigami.Units.largeSpacing
                    Layout.fillHeight: true

                    Repeater {
                        model: node.depth

                        delegate: Rectangle {
                            required property int index

                            x: (index + 0.5) * Kirigami.Units.largeSpacing
                            width: 1
                            // Null while the row is torn down (the tree reloads).
                            height: parent ? parent.height : 0
                            color: Kirigami.Theme.disabledTextColor
                            opacity: 0.3
                        }
                    }
                }

                Rectangle {
                    visible: node.depth > 0
                    Layout.preferredWidth: Kirigami.Units.smallSpacing
                    Layout.preferredHeight: 1
                    color: Kirigami.Theme.disabledTextColor
                    opacity: 0.3
                }

                Kirigami.Icon {
                    source: node.has_children ? (node.open ? "go-down" : "go-next") : "configure"
                    Layout.preferredWidth: Kirigami.Units.iconSizes.small
                    Layout.preferredHeight: Kirigami.Units.iconSizes.small
                    opacity: node.has_children ? 0.9 : 0.5
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 0

                    Controls.Label {
                        Layout.fillWidth: true
                        textFormat: Text.PlainText
                        text: node.label
                        font.family: "monospace"
                        font.bold: true
                        wrapMode: Text.WrapAnywhere
                    }

                    Controls.Label {
                        Layout.fillWidth: true
                        visible: text.length > 0
                        textFormat: Text.PlainText
                        text: node.is_option && node.summary.length > 0 ? node.summary : qsTr("%1 options").arg(Helpers.count(node.option_count))
                        opacity: 0.75
                        font: Kirigami.Theme.smallFont
                        elide: Text.ElideRight
                        maximumLineCount: 1
                        wrapMode: Text.Wrap
                    }

                    Flow {
                        Layout.fillWidth: true
                        Layout.topMargin: Kirigami.Units.smallSpacing
                        visible: node.is_option
                        spacing: Kirigami.Units.smallSpacing

                        Tag {
                            visible: node.value.length > 0
                            text: "= " + node.value
                            tone: "info"
                        }

                        Tag {
                            visible: node.user_set
                            text: qsTr("set in config")
                            tone: "positive"
                        }
                    }
                }

                Kirigami.Icon {
                    visible: !node.has_children
                    source: "go-next"
                    Layout.preferredWidth: Kirigami.Units.iconSizes.small
                    Layout.preferredHeight: Kirigami.Units.iconSizes.small
                    opacity: 0.5
                }
            }

            onClicked: {
                tree.currentIndex = node.index;
                if (node.has_children)
                    page.treeModel.toggle_folder(node.name);
                else
                    page.app.showOption(node.name, page.options);
            }
        }
    }

    TreeSearchResults {
        visible: page.query !== "" && page.options.size > 0
        app: page.app
        options: page.options
        scope: page.scope
        homePrefix: page.homePrefix
        model: page.found ? page.found.rows : []
    }

    Kirigami.PlaceholderMessage {
        anchors.centerIn: parent
        width: parent.width - Kirigami.Units.gridUnit * 4
        visible: page.system.target_error.length > 0
        icon.name: "data-error"
        text: qsTr("No system selected")
        explanation: page.system.target_error

        helpfulAction: Kirigami.Action {
            text: qsTr("Open Settings")
            icon.name: "settings-configure"

            onTriggered: page.app.showSection("settings")
        }
    }
}
