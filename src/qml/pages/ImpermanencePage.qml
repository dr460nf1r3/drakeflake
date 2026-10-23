pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../main/helpers.js" as Helpers
import "impermanence"

Kirigami.Page {
    id: page

    required property var app
    readonly property var system: page.app.systemModel
    readonly property var changes: page.app.changeModel
    readonly property var model: page.app.persistenceModel
    readonly property var userList: page.system.home_users
    property string message: ""
    property string scopeUser: ""
    property string dismissedScanError: ""
    readonly property string notice: page.message.length > 0 ? page.message : page.model.scan_error === page.dismissedScanError ? "" : page.model.scan_error

    function stage(change: var): void {
        if (change.error) {
            page.message = change.error;
            return;
        }

        page.message = "";

        if (change.input === "")
            page.changes.stage_reset(change.name);
        else
            page.message = page.changes.stage_setting(change.name, change.type, change.input, null);
    }

    function syncChanges(): void {
        page.model.set_changes(page.changes.saved_json(), page.changes.plan_json());
    }

    function syncScope(index: int): void {
        page.scopeUser = index <= 0 ? "" : page.userList[index - 1];
        page.model.set_scope(page.scopeUser === "" ? "system" : "home", page.scopeUser);
    }

    function scan(): void {
        page.dismissedScanError = "";
        page.model.scan(page.model.snapshot_offered ? snapshotField.text : "");
    }

    function dismissNotice(): void {
        if (page.message.length > 0)
            page.message = "";
        else
            page.dismissedScanError = page.model.scan_error;
    }

    title: qsTr("Impermanence")

    actions: [
        Kirigami.Action {
            text: qsTr("Reload")
            icon.name: "view-refresh"
            enabled: !page.model.loading
            tooltip: qsTr("Evaluate the persisted folders and files of your flake again")

            onTriggered: page.model.load()
        }
    ]
    header: Controls.Control {
        padding: Kirigami.Units.largeSpacing
        visible: page.model.available

        background: Rectangle {
            color: page.app.vibrancy ? "transparent" : Kirigami.Theme.backgroundColor

            Kirigami.Separator {
                anchors {
                    left: parent.left
                    right: parent.right
                    bottom: parent.bottom
                }
            }
        }
        contentItem: Flow {
            spacing: Kirigami.Units.smallSpacing

            Controls.ComboBox {
                objectName: "impermanenceScope"
                visible: page.userList.length > 0
                model: [qsTr("System")].concat(page.userList.map(user => qsTr("Home Manager: %1").arg(user)))
                currentIndex: page.userList.indexOf(page.scopeUser) + 1

                onActivated: index => page.syncScope(index)
            }

            Controls.Label {
                height: storageBox.height
                verticalAlignment: Text.AlignVCenter
                text: qsTr("Store in:")
            }

            Controls.ComboBox {
                id: storageBox

                model: page.model.storages
                currentIndex: page.model.storages.indexOf(page.model.storage)

                onActivated: page.model.set_storage(storageBox.currentText)
            }

            Controls.TextField {
                id: snapshotField

                objectName: "impermanenceSnapshot"
                visible: page.model.snapshot_offered
                width: Kirigami.Units.gridUnit * 12
                placeholderText: qsTr("Blank snapshot (btrfs)")
                Controls.ToolTip.visible: snapshotField.hovered
                Controls.ToolTip.text: qsTr("Subvolume your btrfs rollback restores, e.g. root-blank. Files in it come back after a reboot.")
            }

            Controls.Button {
                objectName: "impermanenceScan"
                text: page.model.scanned ? qsTr("Scan again") : qsTr("Scan")
                icon.name: "search"
                enabled: !page.model.scanning
                Controls.ToolTip.visible: hovered
                Controls.ToolTip.text: qsTr("List what a reboot removes from the root file system. Asks for the administrator password.")

                onClicked: page.scan()
            }
        }
    }

    onUserListChanged: {
        if (page.scopeUser !== "" && !page.userList.includes(page.scopeUser))
            page.syncScope(0);
    }
    Component.onCompleted: {
        page.syncChanges();

        if (!page.model.loaded && !page.model.loading)
            page.model.load();
    }

    Connections {
        function onChanged(): void {
            page.syncChanges();
        }

        target: page.changes
    }

    ColumnLayout {
        anchors.centerIn: parent
        width: parent.width - Kirigami.Units.gridUnit * 4
        visible: page.model.loading && !page.model.loaded
        spacing: Kirigami.Units.largeSpacing

        Controls.BusyIndicator {
            Layout.alignment: Qt.AlignHCenter
            running: visible
        }

        Controls.Label {
            Layout.fillWidth: true
            horizontalAlignment: Text.AlignHCenter
            textFormat: Text.PlainText
            text: qsTr("Reading the persisted folders and files…")
        }
    }

    Kirigami.PlaceholderMessage {
        objectName: "impermanenceMissing"
        anchors.centerIn: parent
        width: parent.width - Kirigami.Units.gridUnit * 4
        visible: page.model.loaded && !page.model.available
        icon.name: "edit-clear-history"
        text: page.model.error.length > 0 ? qsTr("The persisted folders could not be read") : qsTr("This host does not use impermanence")
        explanation: page.model.error.length > 0 ? Helpers.richText(page.model.error) : qsTr("Import the impermanence module (nixosModules.impermanence) in your flake. This page then lists what your host keeps and what a reboot removes.")
    }

    ColumnLayout {
        anchors.fill: parent
        visible: page.model.available
        spacing: Kirigami.Units.smallSpacing

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            objectName: "impermanenceNotice"
            visible: page.notice.length > 0
            type: Kirigami.MessageType.Error
            text: Helpers.richText(page.notice)

            // The built-in close button writes `visible` and ends its binding.
            actions: [
                Kirigami.Action {
                    text: qsTr("Dismiss")
                    icon.name: "dialog-close"
                    displayHint: Kirigami.DisplayHint.IconOnly

                    onTriggered: page.dismissNotice()
                }
            ]
        }

        GridLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            columns: page.width > Kirigami.Units.gridUnit * 36 ? 2 : 1
            columnSpacing: Kirigami.Units.largeSpacing
            rowSpacing: Kirigami.Units.largeSpacing

            PersistedList {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                model: page.model

                onUnpersist: (option, entry) => page.stage(page.model.unpersist(option, entry))
                onKeep: (option, entry) => page.stage(page.model.keep(option, entry))
            }

            EphemeralList {
                Layout.fillWidth: true
                Layout.fillHeight: true
                Layout.preferredWidth: 1
                model: page.model

                onPersist: (path, kind) => page.stage(page.model.persist(path, kind))
            }
        }
    }
}
