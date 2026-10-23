pragma ComponentBehavior: Bound

import QtQuick

import "helpers.js" as Helpers

Item {
    id: shortcuts

    required property var app

    Shortcut {
        sequence: StandardKey.Find

        onActivated: Helpers.focusSearch(shortcuts.app.pageStack.currentItem)
    }

    Shortcut {
        sequence: shortcuts.app.macos ? "Meta+[" : "Ctrl+["

        onActivated: Helpers.goBack(shortcuts.app.pageStack)
    }

    Shortcut {
        sequence: "Escape"

        onActivated: Helpers.goBack(shortcuts.app.pageStack)
    }

    Shortcut {
        sequence: StandardKey.Save

        onActivated: Helpers.applyPrimary(shortcuts.app)
    }

    Shortcut {
        sequence: StandardKey.HelpContents

        onActivated: shortcuts.app.showHelp(shortcuts.app.section)
    }
}
