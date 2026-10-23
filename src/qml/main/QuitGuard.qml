pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

Item {
    id: guard

    required property var app
    property bool quitting: false
    property bool confirmed: false
    property bool busy: guard.app.systemModel.running
    readonly property alias dialog: dialog

    function intercept(close: var): void {
        if (guard.confirmed || !guard.busy)
            return;
        close.accepted = false;
        if (!guard.quitting)
            dialog.open();
    }

    function keepRunning(): void {
        dialog.close();
    }

    function quit(): void {
        dialog.close();
        guard.quitting = true;
        guard.app.systemModel.cancel();

        // Longer than the cancel escalation (Ctrl-C, SIGTERM, SIGKILL), so a stubborn tool is stopped before the app goes.
        giveUp.restart();
    }

    function finish(): void {
        giveUp.stop();
        guard.confirmed = true;
        guard.app.close();
    }

    Connections {
        function onState_changed(): void {
            if (guard.quitting && !guard.busy)
                guard.finish();
        }

        target: guard.app.systemModel
    }

    Timer {
        id: giveUp

        interval: 9000

        onTriggered: guard.finish()
    }

    Kirigami.PromptDialog {
        id: dialog

        title: qsTr("Operation still running")
        subtitle: qsTr("An operation is still running. Quit anyway?")
        standardButtons: Kirigami.Dialog.NoButton

        customFooterActions: [
            Kirigami.Action {
                text: qsTr("Quit")
                icon.name: "process-stop"

                onTriggered: guard.quit()
            },
            Kirigami.Action {
                text: qsTr("Keep running")
                icon.name: "dialog-cancel"

                onTriggered: guard.keepRunning()
            }
        ]
    }
}
