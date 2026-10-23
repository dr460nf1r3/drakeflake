pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../components"
import "helpers.js" as Helpers

Kirigami.GlobalDrawer {
    id: drawer

    required property string section
    required property real windowWidth
    required property bool generations
    required property int changeCount
    required property bool running
    required property bool macos
    required property bool unifiedTitlebar
    required property real toolBarHeight
    required property bool vibrancy

    signal sectionChosen(string name)

    function leaf(section: string): var {
        for (const action of drawer.actions) {
            if (action.text !== undefined && action.section === section)
                return action;
        }

        return null;
    }

    isMenu: false
    // Collapses to icons by the window width, since wideScreen depends on the drawer's own width.
    modal: false
    collapsed: drawer.windowWidth < Kirigami.Units.gridUnit * 45
    preferredSize: Kirigami.Units.gridUnit * 13
    collapsible: true
    showHeaderWhenCollapsed: true

    header: Kirigami.AbstractApplicationHeader {
        id: drawerHeader

        readonly property Controls.ItemDelegate entry: Controls.ItemDelegate {
        }
        readonly property real iconOffset: (Kirigami.Units.iconSizes.medium - Kirigami.Units.iconSizes.smallMedium) / 2
        readonly property real headerIconSize: drawer.collapsed ? Kirigami.Units.iconSizes.medium : Kirigami.Units.iconSizes.large

        preferredHeight: Math.max(drawer.toolBarHeight, drawerHeader.headerIconSize + Kirigami.Units.smallSpacing * 2)
        topPadding: bottomPadding
        separatorVisible: !drawer.macos

        contentItem: RowLayout {
            anchors.fill: parent
            anchors.leftMargin: drawer.macos ? Kirigami.Units.largeSpacing : drawerHeader.entry.leftPadding - drawerHeader.iconOffset - drawerHeader.leftPadding
            spacing: drawer.macos ? Kirigami.Units.smallSpacing : Kirigami.Units.largeSpacing - drawerHeader.iconOffset

            Logo {
                Layout.preferredWidth: drawerHeader.headerIconSize
                Layout.preferredHeight: drawerHeader.headerIconSize
            }

            Kirigami.Heading {
                level: 2
                text: qsTr("DrakeFlake")
                visible: !drawer.collapsed
                wrapMode: Text.NoWrap
                fontSizeMode: Text.HorizontalFit
                minimumPointSize: Kirigami.Theme.smallFont.pointSize
                Layout.fillWidth: true
                Layout.minimumWidth: implicitWidth
            }
        }
    }
    actions: [
        Kirigami.Action {
            property string section: "overview"

            text: qsTr("Overview")
            icon.name: "go-home"
            checkable: true
            checked: drawer.section === "overview"
            tooltip: qsTr("This system, a search over everything and the main sections")

            onTriggered: drawer.sectionChosen("overview")
        },
        Kirigami.Action {
            property string section: "options"

            text: qsTr("Options")
            icon.name: "view-list-details"
            checkable: true
            checked: drawer.section === "options"

            onTriggered: drawer.sectionChosen("options")
        },
        Kirigami.Action {
            property string section: "home"

            text: qsTr("Home Manager")
            icon.name: "user"
            checkable: true
            checked: drawer.section === "home"
            tooltip: qsTr("Home Manager options of the users on this host")

            onTriggered: drawer.sectionChosen("home")
        },
        Kirigami.Action {
            property string section: "packages"

            text: qsTr("Packages")
            icon.name: "package"
            checkable: true
            checked: drawer.section === "packages"

            onTriggered: drawer.sectionChosen("packages")
        },
        Kirigami.Action {
            property string section: "functions"

            text: qsTr("Functions")
            icon.name: "code-function"
            checkable: true
            checked: drawer.section === "functions"
            tooltip: qsTr("Nix functions of lib, builtins and pkgs with their documentation, searched offline")

            onTriggered: drawer.sectionChosen("functions")
        },
        Kirigami.Action {
            property string section: "generations"

            text: qsTr("Generations")
            icon.name: "view-history"
            visible: drawer.generations
            checkable: true
            checked: drawer.section === "generations"
            tooltip: qsTr("Earlier versions of your system: compare, test or go back")

            onTriggered: drawer.sectionChosen("generations")
        },
        Kirigami.Action {
            property string section: "tree"

            text: qsTr("Tree View")
            icon.name: "view-list-tree"
            checkable: true
            checked: drawer.section === "tree"
            tooltip: qsTr("Browse every option as a tree, including defaults")

            onTriggered: drawer.sectionChosen("tree")
        },
        Kirigami.Action {
            property string section: "configured"

            text: qsTr("Configured")
            icon.name: "dialog-ok"
            checkable: true
            checked: drawer.section === "configured"
            tooltip: qsTr("Everything your host's configuration currently sets")

            onTriggered: drawer.sectionChosen("configured")
        },
        Kirigami.Action {
            separator: true
        },
        Kirigami.Action {
            property string section: "actions"

            text: qsTr("Actions")
            icon.name: "system-run"
            checkable: true
            checked: drawer.section === "actions"
            tooltip: qsTr("One-off tasks: create images of this system")

            onTriggered: drawer.sectionChosen("actions")
        },
        Kirigami.Action {
            property string section: "cleanup"

            text: qsTr("Cleanup")
            icon.name: "edit-clear-all"
            checkable: true
            checked: drawer.section === "cleanup"
            tooltip: qsTr("Free disk space: old generations and unused packages")

            onTriggered: drawer.sectionChosen("cleanup")
        },
        Kirigami.Action {
            property string section: "inputs"

            text: qsTr("Inputs")
            icon.name: "download"
            checkable: true
            checked: drawer.section === "inputs"
            tooltip: qsTr("Flake inputs from flake.lock: sources, revisions and updates")

            onTriggered: drawer.sectionChosen("inputs")
        },
        Kirigami.Action {
            property string section: "impermanence"

            text: qsTr("Impermanence")
            icon.name: "edit-clear-history"
            visible: Helpers.sectionOffered("impermanence", drawer.macos)
            checkable: true
            checked: drawer.section === "impermanence"
            tooltip: qsTr("What survives a reboot and what is removed")

            onTriggered: drawer.sectionChosen("impermanence")
        },
        Kirigami.Action {
            property string section: "history"

            text: qsTr("History")
            icon.name: "document-revert"
            checkable: true
            checked: drawer.section === "history"
            tooltip: qsTr("Earlier saved settings: see the differences, restore one")

            onTriggered: drawer.sectionChosen("history")
        },
        Kirigami.Action {
            separator: true
        },
        Kirigami.Action {
            property string section: "changes"

            text: drawer.changeCount > 0 ? qsTr("Changes (%1)").arg(drawer.changeCount) : qsTr("Changes")
            icon.name: drawer.changeCount > 0 ? "document-save" : "document-edit"
            checkable: true
            checked: drawer.section === "changes"

            onTriggered: drawer.sectionChosen("changes")
        },
        Kirigami.Action {
            property string section: "log"

            text: drawer.running ? qsTr("Console (running)") : qsTr("Console")
            icon.name: "dialog-scripts"
            checkable: true
            checked: drawer.section === "log"

            onTriggered: drawer.sectionChosen("log")
        },
        Kirigami.Action {
            separator: true
        },
        Kirigami.Action {
            property string section: "settings"

            text: qsTr("Settings")
            icon.name: "configure"
            checkable: true
            checked: drawer.section === "settings"

            onTriggered: drawer.sectionChosen("settings")
        },
        Kirigami.Action {
            property string section: "docs"

            text: qsTr("Documentation")
            icon.name: "documentation"
            checkable: true
            checked: drawer.section === "docs"
            tooltip: qsTr("The DrakeFlake handbook: how to use each page")

            onTriggered: drawer.sectionChosen("docs")
        },
        Kirigami.Action {
            property string section: "about"

            text: qsTr("About")
            icon.name: "help-hint"
            checkable: true
            checked: drawer.section === "about"

            onTriggered: drawer.sectionChosen("about")
        }
    ]

    Component.onCompleted: {
        if (drawer.macos)
            // A visual parent: the drawer itself is a popup, not an item.
            drawer.background = plainBackground.createObject(drawer.contentItem);
    }

    Component {
        id: plainBackground

        Rectangle {
            color: drawer.vibrancy ? "transparent" : Kirigami.Theme.backgroundColor
        }
    }

    Binding {
        when: drawer.vibrancy
        target: drawerHeader.background
        property: "color"
        value: "transparent"
    }
}
