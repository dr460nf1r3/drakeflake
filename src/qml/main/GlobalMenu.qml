pragma ComponentBehavior: Bound

import QtQuick
import Qt.labs.platform as Platform

Platform.MenuBar {
    id: bar

    required property var source

    Instantiator {
        model: bar.source.menus

        delegate: Platform.Menu {
            id: submenu

            required property var modelData

            title: submenu.modelData.title

            Instantiator {
                model: submenu.modelData.count

                delegate: Platform.MenuItem {
                    id: item

                    required property int index
                    readonly property var entry: submenu.modelData.itemAt(item.index)
                    readonly property var action: item.entry.action ?? null

                    // A MenuSeparator has no action property at all.
                    separator: item.entry.action === undefined
                    text: item.action ? item.action.text : (item.entry.text ?? "")
                    shortcut: item.action ? item.action.shortcut : ""
                    enabled: item.action ? item.action.enabled : item.entry.enabled
                    // `visible` of a closed menu's entries is always false.
                    visible: item.entry.shown ?? true

                    onTriggered: {
                        if (item.action)
                            item.action.trigger();
                    }
                }

                onObjectAdded: (index, object) => submenu.insertItem(index, object)
                onObjectRemoved: (index, object) => submenu.removeItem(object)
            }
        }

        onObjectAdded: (index, object) => bar.insertMenu(index, object)
        onObjectRemoved: (index, object) => bar.removeMenu(object)
    }
}
