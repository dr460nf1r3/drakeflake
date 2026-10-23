pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window

// An item, not an object: the timers and connections below need a default property.
Item {
    id: windowState

    required property var app
    // The screen's available size, read where `Screen` resolves (the window).
    required property real availableWidth
    required property real availableHeight

    function restore(): void {
        const settings = windowState.app.settingsModel;

        if (windowState.app.programOutdated || !settings)
            return;

        const saved = settings.window_geometry;

        if (saved.width == null || saved.height == null)
            return;

        if (saved.width < windowState.app.minimumWidth || saved.height < windowState.app.minimumHeight)
            return;

        if (saved.width > windowState.availableWidth || saved.height > windowState.availableHeight)
            return;

        windowState.app.width = saved.width;
        windowState.app.height = saved.height;

        if (saved.x == null || saved.y == null)
            return;

        if (saved.x < 0 || saved.y < 0)
            return;

        if (saved.x + windowState.app.minimumWidth > windowState.availableWidth)
            return;

        if (saved.y + windowState.app.minimumHeight > windowState.availableHeight)
            return;

        windowState.app.x = saved.x;
        windowState.app.y = saved.y;
    }

    function save(): void {
        if (windowState.app.programOutdated || windowState.app.visibility !== Window.Windowed)
            return;

        windowState.app.settingsModel.save_window_geometry(windowState.app.x, windowState.app.y, windowState.app.width, windowState.app.height);
    }

    Component.onCompleted: windowState.restore()

    Timer {
        id: saver

        interval: 1500

        onTriggered: windowState.save()
    }

    Connections {
        function onXChanged(): void {
            saver.restart();
        }

        function onYChanged(): void {
            saver.restart();
        }

        function onWidthChanged(): void {
            saver.restart();
        }

        function onHeightChanged(): void {
            saver.restart();
        }

        target: windowState.app
    }
}
