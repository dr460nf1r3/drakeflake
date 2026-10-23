pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../components"
import "optiondetails"
import "../main/helpers.js" as Helpers
import "../main/kinds.js" as Kinds

Kirigami.ScrollablePage {
    id: page

    required property var app
    property var options: page.app.optionModel
    readonly property var changes: page.app.changeModel
    readonly property bool isOptionDetails: true
    property string optionName: ""
    readonly property var info: {
        page.options.revision;
        page.changes.revision;

        return page.options.details(page.optionName);
    }
    readonly property bool known: page.info.name !== undefined
    readonly property bool editable: page.known && !page.info.readOnly && !page.info.templated
    readonly property string stagedState: {
        page.changes.revision;
        return page.changes.option_state(page.optionName);
    }
    readonly property string savedInput: {
        page.changes.revision;

        const staged = page.changes.staged_input(page.optionName);
        if (page.stagedState !== "" && page.stagedState !== "reset")
            return staged;
        return page.info.valueInput !== undefined && page.info.valueInput !== null ? page.info.valueInput : "";
    }
    readonly property var stagedSettings: {
        page.changes.revision;
        return page.stagedState === "set" || page.stagedState === "managed" ? page.changes.staged_settings(page.optionName) : {};
    }
    readonly property string checkStatus: {
        page.changes.revision;
        return page.stagedState === "set" ? page.changes.check_status(page.optionName) : "";
    }
    readonly property string checkMessage: {
        page.changes.revision;
        return page.changes.check_message(page.optionName);
    }
    property string editText: ""
    property bool dirty: false
    property string stageError: ""
    property string inputMode: "typed"
    property string priority: ""
    property string scopeHost: ""
    readonly property bool expressionMode: page.inputMode === "expression"
    readonly property string validation: {
        if (!page.editable || !(page.dirty || page.editText.length > 0))
            return "";
        return page.expressionMode ? page.changes.validate_expression(page.editText) : page.changes.validate(page.info.type, page.editText);
    }
    readonly property var priorityInfo: {
        if (!page.known)
            return {
                choices: [],
                implicit: ""
            };
        return page.changes.priority_choices(page.info.type, page.editText, page.inputMode);
    }
    readonly property string shownPriority: page.priority.length > 0 ? page.priority : page.priorityInfo.implicit
    readonly property bool hostScopeAllowed: page.known && page.changes.host_scope_allowed(page.optionName)
    readonly property var hostInfo: {
        page.app.optionModel.revision;

        return page.app.optionModel.details("networking.hostName");
    }
    readonly property string currentHost: page.hostInfo.valueState === Kinds.valueReady && typeof page.hostInfo.valueInput === "string" ? page.hostInfo.valueInput : ""

    function resetEditor(): void {
        page.editText = page.savedInput;
        const settings = page.stagedState === "set" || page.stagedState === "managed" ? page.changes.staged_settings(page.optionName) : {};
        page.inputMode = settings.mode || "typed";
        page.priority = settings.priority || "";
        page.scopeHost = settings.host || "";
        page.dirty = false;
        page.stageError = "";
    }

    function editSettings(mode: string, priority: string, host: string): void {
        page.inputMode = mode;
        page.priority = priority;
        page.scopeHost = host;
        page.dirty = true;
        page.stageError = "";
    }

    function requestHost(): void {
        if (page.hostScopeAllowed && page.hostInfo.valueState === Kinds.valueUnknown)
            page.app.optionModel.request_value("networking.hostName");
    }

    function edit(text: string): void {
        page.editText = text;
        page.dirty = true;
        page.stageError = "";
    }

    function stage(): void {
        const settings = {
            mode: page.inputMode,
            priority: page.priority,
            host: page.scopeHost
        };
        page.stageError = page.changes.stage_setting(page.info.name, page.info.type, page.editText, settings);
        if (page.stageError === "")
            page.dirty = false;
    }

    // Deferred: right after a name change, `info` still describes the
    // previous option until its binding updates.
    function requestValueIfUnknown(): void {
        if (page.known && page.info.valueState === Kinds.valueUnknown)
            page.options.request_value(page.optionName);
    }

    title: page.known ? page.info.name.split(".").slice(-1)[0] : qsTr("Option")

    actions: [
        Kirigami.Action {
            text: qsTr("Copy name")
            icon.name: "edit-copy"
            displayHint: Kirigami.DisplayHint.IconOnly
            tooltip: qsTr("Copy the option name")

            onTriggered: header.copyName()
        },
        Kirigami.Action {
            text: qsTr("Re-evaluate")
            icon.name: "view-refresh"
            displayHint: Kirigami.DisplayHint.IconOnly
            tooltip: qsTr("Evaluate the current value again")
            enabled: page.known && page.info.valueState !== Kinds.valueLoading

            onTriggered: page.options.request_value(page.optionName)
        },
        Kirigami.Action {
            text: qsTr("Close")
            icon.name: "dialog-close"
            displayHint: Kirigami.DisplayHint.IconOnly

            onTriggered: page.app.pageStack.removePage(page)
        }
    ]

    onOptionNameChanged: {
        page.resetEditor();
        Qt.callLater(page.requestHost);

        Qt.callLater(page.requestValueIfUnknown);
    }
    onSavedInputChanged: if (!page.dirty)
        page.resetEditor()

    // Imperative on purpose: editText is user-editable, so a binding to
    // savedInput would be broken (or overwrite typing) on the first edit.
    Component.onCompleted: {
        page.resetEditor();
        Qt.callLater(page.requestHost);

        Qt.callLater(page.requestValueIfUnknown);
    }

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        Kirigami.PlaceholderMessage {
            Layout.fillWidth: true
            visible: !page.known
            icon.name: "dialog-question"
            text: qsTr("Option not found")
            explanation: page.optionName
        }

        OptionHeader {
            id: header

            Layout.fillWidth: true
            visible: page.known
            app: page.app
            optionName: page.optionName
            info: page.info
            stagedState: page.stagedState
            stagedSettings: page.stagedSettings
        }

        CurrentValueSection {
            Layout.fillWidth: true
            visible: page.known
            app: page.app
            info: page.info

            onEvaluateRequested: page.options.request_value(page.optionName)
            onOpenLocation: (file, line) => page.app.optionModel.open_in_editor(file, line)
        }

        SectionHeading {
            visible: page.known
            text: qsTr("Change")
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: page.known && !page.editable
            type: Kirigami.MessageType.Information
            // Escaped: "<name>" would otherwise be read as a tag and vanish.
            text: Helpers.richText(page.info.templated ? qsTr("This option belongs to a named entry (<name>). Search for the concrete name, e.g. users.users.alice.…, or set it in your own configuration.") : qsTr("This option is read-only and cannot be set."))
        }

        SettingEditor {
            Layout.fillWidth: true
            visible: page.editable
            optionName: page.optionName
            editable: page.editable
            info: page.info
            editText: page.editText
            inputMode: page.inputMode
            priority: page.priority
            priorityInfo: page.priorityInfo
            shownPriority: page.shownPriority
            hostScopeAllowed: page.hostScopeAllowed
            scopeHost: page.scopeHost
            currentHost: page.currentHost
            hostState: page.hostInfo.valueState || ""

            onEdited: text => page.edit(text)
            onSettingsEdited: (mode, priority, host) => page.editSettings(mode, priority, host)
        }

        ChangeActions {
            Layout.fillWidth: true
            app: page.app
            editable: page.editable
            expressionMode: page.expressionMode
            kind: page.info.kind || ""
            validation: page.validation
            stageError: page.stageError
            dirty: page.dirty
            stagedState: page.stagedState
            checkStatus: page.checkStatus
            checkMessage: page.checkMessage

            onStageRequested: page.stage()
            onRevertRequested: page.resetEditor()
            onUnstageRequested: {
                page.changes.unstage("option:" + page.optionName);
                page.resetEditor();
            }
            onRemoveRequested: page.changes.stage_reset(page.optionName)
        }

        ReferenceSection {
            Layout.fillWidth: true
            app: page.app
            info: page.known ? page.info : ({})
        }

        Item {
            Layout.fillHeight: true
        }
    }
}
