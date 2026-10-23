pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

import "inputs"
import "../main/kinds.js" as Kinds

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var system: page.app.systemModel
    readonly property var inputsModel: page.app.inputsModel
    readonly property var inputs: page.inputsModel.inputs
    readonly property var updatable: page.inputs.filter(input => input.check && input.check.state === "available")
    property bool updating: false
    property bool updated: false
    property string message: ""
    property var expanded: ({})

    function toggle(name: string): void {
        const next = Object.assign({}, page.expanded);
        next[name] = !next[name];
        page.expanded = next;
    }

    function update(names: var): void {
        page.updated = false;
        page.message = page.system.update_lock(names);
        page.updating = page.message.length === 0;
    }

    function rebuild(mode: string): void {
        page.message = page.system.rebuild(mode);

        if (page.message.length === 0) {
            page.updated = false;
            page.app.showSection("log");
        }
    }

    title: qsTr("Inputs")

    actions: [
        Kirigami.Action {
            text: qsTr("Cancel")
            icon.name: "process-stop"
            visible: page.updating
            tooltip: qsTr("Stop the running update (e.g. when a download hangs)")

            onTriggered: page.system.cancel()
        },
        Kirigami.Action {
            text: qsTr("Check for updates")
            icon.name: "view-refresh"
            enabled: !page.inputsModel.checking && !page.system.running
            tooltip: qsTr("Ask each source for its newest revision")

            onTriggered: page.inputsModel.load(true)
        },
        Kirigami.Action {
            text: page.updatable.length > 0 ? qsTr("Update all (%1)").arg(page.updatable.length) : qsTr("Update all")
            icon.name: "update-none"
            enabled: !page.system.running && page.inputs.length > 0
            tooltip: qsTr("Update every input to its newest revision (nix flake update)")

            onTriggered: page.update([])
        }
    ]

    Component.onCompleted: page.inputsModel.load(true)

    Connections {
        function onState_changed(): void {
            if (page.updating && !page.system.running) {
                page.updating = false;
                page.updated = page.system.last_result === Kinds.outcomeOk;

                if (!page.updated)
                    page.message = qsTr("The update failed. See the Console for details.");
            }
        }

        target: page.system
    }

    ListView {
        id: list

        model: page.inputs
        spacing: 0

        header: InputsHeader {
            width: ListView.view ? ListView.view.width : 0
            app: page.app
            error: page.inputsModel.error
            message: page.message
            updated: page.updated
            busy: page.system.running
            checking: page.inputsModel.checking
            inputCount: page.inputs.length
            updatableCount: page.updatable.length

            onRebuildRequested: mode => page.rebuild(mode)
        }
        delegate: InputRow {
            id: row

            width: ListView.view.width
            app: page.app
            checking: page.inputsModel.checking
            busy: page.system.running
            open: !!page.expanded[row.modelData.name]

            onToggled: page.toggle(row.modelData.name)
            onUpdateRequested: page.update([row.modelData.name])
        }

        Kirigami.PlaceholderMessage {
            anchors.centerIn: parent
            width: parent.width - Kirigami.Units.gridUnit * 4
            visible: list.count === 0
            icon.name: page.inputsModel.error.length > 0 ? "data-error" : "package-x-generic"
            text: page.inputsModel.error.length > 0 ? qsTr("Cannot read flake.lock") : qsTr("No flake inputs")
            explanation: page.inputsModel.error.length > 0 ? page.inputsModel.error : qsTr("This flake has no inputs, or no lock file yet.")
        }
    }
}
