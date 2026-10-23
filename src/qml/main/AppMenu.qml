pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls

import "helpers.js" as Helpers

Controls.MenuBar {
    id: menu

    required property var app
    required property bool generations
    // Hardcoded "Ctrl+" sequences would need the physical control key on a Mac.
    readonly property string mod: menu.app.macos ? "Meta" : "Ctrl"
    readonly property var focusItem: menu.app ? menu.app.activeFocusItem : null
    readonly property alias aboutAction: aboutAction
    readonly property alias fullscreenAction: fullscreenAction
    readonly property alias findAction: findAction
    readonly property alias backAction: backAction
    readonly property alias saveAction: saveAction
    readonly property alias helpAction: helpAction
    property bool hiddenForShot: false
    readonly property bool exported: menu.app.settingsModel.global_menu === true
    // The only reference to the parentless global menu.
    property var globalMenu: null

    function canEdit(name: string): bool {
        return menu.focusItem !== null && typeof menu.focusItem[name] === "function";
    }

    function editText(name: string): void {
        if (menu.canEdit(name))
            menu.focusItem[name]();
    }

    // Without a parent its shortcuts match no window, so each key fires only this bar's action.
    function mirror(): var {
        const component = Qt.createComponent("GlobalMenu.qml");
        const mirrored = component.createObject(null, {
            source: menu,
            window: menu.app
        });
        if (mirrored === null)
            console.warn("global menu failed: " + component.errorString());
        return mirrored;
    }

    visible: !menu.exported && !menu.hiddenForShot

    Component.onCompleted: {
        // Once the window exists, which the global menu attaches to.
        if (menu.exported)
            Qt.callLater(() => menu.globalMenu = menu.mirror());
    }

    Controls.Menu {
        title: qsTr("File")

        Controls.Action {
            id: saveAction

            text: qsTr("Save Changes")
            shortcut: StandardKey.Save
            enabled: menu.app.changeModel.count > 0

            onTriggered: Helpers.applyPrimary(menu.app)
        }

        Controls.Action {
            text: qsTr("Quit DrakeFlake")
            shortcut: StandardKey.Quit

            onTriggered: menu.app.close()
        }
    }

    Controls.Menu {
        title: qsTr("Edit")

        Controls.Action {
            text: qsTr("Undo")
            shortcut: StandardKey.Undo
            enabled: menu.canEdit("undo") && menu.focusItem.canUndo === true

            onTriggered: menu.editText("undo")
        }

        Controls.Action {
            text: qsTr("Redo")
            shortcut: StandardKey.Redo
            enabled: menu.canEdit("redo") && menu.focusItem.canRedo === true

            onTriggered: menu.editText("redo")
        }

        Controls.MenuSeparator {
        }

        Controls.Action {
            text: qsTr("Cut")
            shortcut: StandardKey.Cut
            enabled: menu.canEdit("cut") && menu.focusItem.readOnly !== true && menu.focusItem.selectedText !== ""

            onTriggered: menu.editText("cut")
        }

        Controls.Action {
            text: qsTr("Copy")
            shortcut: StandardKey.Copy
            enabled: menu.canEdit("copy") && menu.focusItem.selectedText !== ""

            onTriggered: menu.editText("copy")
        }

        Controls.Action {
            text: qsTr("Paste")
            shortcut: StandardKey.Paste
            enabled: menu.canEdit("paste") && menu.focusItem.canPaste === true

            onTriggered: menu.editText("paste")
        }

        Controls.Action {
            text: qsTr("Select All")
            shortcut: StandardKey.SelectAll
            enabled: menu.canEdit("selectAll")

            onTriggered: menu.editText("selectAll")
        }

        Controls.Action {
            id: findAction

            text: qsTr("Find in Page")
            shortcut: StandardKey.Find
            enabled: typeof menu.app.pageStack.currentItem?.focusSearch === "function"

            onTriggered: Helpers.focusSearch(menu.app.pageStack.currentItem)
        }

        Controls.MenuSeparator {
        }

        Controls.Action {
            text: qsTr("Settings…")
            shortcut: StandardKey.Preferences

            onTriggered: menu.app.showSection("settings")
        }
    }

    Controls.Menu {
        title: qsTr("View")

        Controls.Action {
            text: qsTr("Overview")
            shortcut: menu.mod + "+1"

            onTriggered: menu.app.showSection("overview")
        }

        Controls.Action {
            text: qsTr("Options")
            shortcut: menu.mod + "+2"

            onTriggered: menu.app.showSection("options")
        }

        Controls.Action {
            text: qsTr("Home Manager")
            shortcut: menu.mod + "+3"

            onTriggered: menu.app.showSection("home")
        }

        Controls.Action {
            text: qsTr("Packages")
            shortcut: menu.mod + "+4"

            onTriggered: menu.app.showSection("packages")
        }

        Controls.Action {
            text: qsTr("Functions")
            shortcut: menu.mod + "+5"

            onTriggered: menu.app.showSection("functions")
        }

        Controls.MenuItem {
            id: generationsItem

            readonly property bool shown: menu.generations

            action: viewGenerations
            visible: generationsItem.shown

            Controls.Action {
                id: viewGenerations

                text: qsTr("Generations")
                shortcut: menu.mod + "+6"

                onTriggered: menu.app.showSection("generations")
            }
        }

        Controls.Action {
            text: qsTr("Actions")
            shortcut: menu.mod + "+7"

            onTriggered: menu.app.showSection("actions")
        }

        Controls.Action {
            text: qsTr("Cleanup")
            shortcut: menu.mod + "+8"

            onTriggered: menu.app.showSection("cleanup")
        }

        Controls.Action {
            text: qsTr("Inputs")
            shortcut: menu.mod + "+9"

            onTriggered: menu.app.showSection("inputs")
        }

        Controls.MenuItem {
            id: impermanenceItem

            readonly property bool shown: Helpers.sectionOffered("impermanence", menu.app.macos)

            action: viewImpermanence
            visible: impermanenceItem.shown

            Controls.Action {
                id: viewImpermanence

                text: qsTr("Impermanence")

                onTriggered: menu.app.showSection("impermanence")
            }
        }

        Controls.Action {
            id: backAction

            text: qsTr("Back")
            shortcut: menu.mod + "+["
            enabled: menu.app.pageStack.depth > 1

            onTriggered: Helpers.goBack(menu.app.pageStack)
        }

        Controls.MenuSeparator {
        }

        Controls.Action {
            text: qsTr("Tree View")

            onTriggered: menu.app.showSection("tree")
        }

        Controls.Action {
            text: qsTr("Changes")

            onTriggered: menu.app.showSection("changes")
        }

        Controls.Action {
            text: qsTr("Configured")

            onTriggered: menu.app.showSection("configured")
        }

        Controls.Action {
            text: qsTr("History")

            onTriggered: menu.app.showSection("history")
        }

        Controls.Action {
            text: qsTr("Console")

            onTriggered: menu.app.showSection("log")
        }

        Controls.Action {
            text: qsTr("Settings")

            onTriggered: menu.app.showSection("settings")
        }

        Controls.Action {
            text: qsTr("About")

            onTriggered: menu.app.showSection("about")
        }
    }

    Controls.Menu {
        title: qsTr("Window")

        Controls.Action {
            text: qsTr("Minimize")
            shortcut: menu.mod + "+M"

            onTriggered: menu.app.showMinimized()
        }

        Controls.Action {
            id: fullscreenAction

            text: qsTr("Toggle Full Screen")
            shortcut: menu.app.macos ? "Ctrl+Meta+F" : "F11"

            onTriggered: menu.app.toggleFullScreen()
        }
    }

    Controls.Menu {
        title: qsTr("Help")

        Controls.Action {
            id: helpAction

            text: qsTr("Help for This Page")
            shortcut: StandardKey.HelpContents

            onTriggered: menu.app.showHelp(menu.app.section)
        }

        Controls.Action {
            text: qsTr("DrakeFlake Handbook")

            onTriggered: menu.app.showHelp("docs")
        }

        Controls.Action {
            id: aboutAction

            objectName: "menuAboutAction"
            text: qsTr("About DrakeFlake")

            onTriggered: menu.app.showSection("about")
        }
    }
}
