pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../main/helpers.js" as Helpers
import "../main/kinds.js" as Kinds

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var system: page.app.systemModel
    readonly property var consoleModel: page.app.consoleModel
    readonly property bool running: page.system.running
    property var askedQuestions: []
    readonly property int terminalColumns: Math.floor((list.width - Kirigami.Units.largeSpacing * 2) / metrics.advanceWidth("M")) - 1
    readonly property int terminalRows: Math.floor(list.height / (metrics.lineSpacing + Kirigami.Units.smallSpacing * 2)) - 1
    readonly property var inputsPage: page.app.sectionPages.inputs
    readonly property bool updateDone: page.inputsPage !== undefined && page.inputsPage !== null && page.inputsPage.updated === true
    readonly property var rebuildModes: page.system.capabilities.rebuild
    readonly property bool updateRebuildOffered: !page.system.running && page.system.last_result === Kinds.outcomeOk && page.updateDone && (page.rebuildModes.includes("switch") || page.rebuildModes.includes("build"))

    function sendTerminalSize(): void {
        if (page.terminalColumns > 0 && page.terminalRows > 0)
            page.consoleModel.set_terminal_size(page.terminalColumns, page.terminalRows);
    }

    function isPromptLine(text: string): bool {
        const line = text.trim();
        const lower = line.toLowerCase();

        if (lower.indexOf("password") >= 0 && /:\s*$/.test(line))
            return true;

        if (/[\(\[]\s*y\s*\/\s*n\s*[\)\]]/i.test(line))
            return true;

        return page.askedQuestions.some(core => core.length > 0 && line.indexOf(core) >= 0);
    }

    function rememberQuestion(): void {
        const core = page.consoleModel.prompt_core;

        if (core.length > 0 && page.askedQuestions.indexOf(core) < 0)
            page.askedQuestions = page.askedQuestions.concat([core]);
    }

    title: qsTr("Console")
    // ScrollablePage turns this into the list's bottom margin: room to scroll
    // past the newest line, which the view keeps at the bottom edge.
    bottomPadding: 0

    actions: [
        Kirigami.Action {
            text: qsTr("Cancel")
            icon.name: "process-stop"
            visible: page.system.running

            onTriggered: page.system.cancel()
        },
        Kirigami.Action {
            text: qsTr("Restore previous file")
            icon.name: "document-revert"
            visible: page.system.can_restore
            tooltip: qsTr("Put drakeflake.nix back as it was before the failed apply")

            onTriggered: page.system.restore_backup()
        },
        Kirigami.Action {
            text: qsTr("Copy")
            icon.name: "edit-copy"
            enabled: page.consoleModel.line_count > 0
            displayHint: Kirigami.DisplayHint.IconOnly
            tooltip: qsTr("Copy the whole output")

            onTriggered: {
                clipboardHelper.text = page.consoleModel.log_text();
                clipboardHelper.selectAll();
                clipboardHelper.copy();
                clipboardHelper.text = "";
            }
        },
        Kirigami.Action {
            text: qsTr("Clear")
            icon.name: "edit-clear-history"
            enabled: !page.system.running && page.consoleModel.line_count > 0
            displayHint: Kirigami.DisplayHint.IconOnly
            tooltip: qsTr("Clear the console")

            onTriggered: page.consoleModel.clear_log()
        }
    ]
    header: ColumnLayout {
        spacing: 0

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            position: Kirigami.InlineMessage.Position.Header
            visible: page.consoleModel.prompt.length > 0 && !page.consoleModel.prompt_password
            type: Kirigami.MessageType.Warning
            text: Helpers.richText(page.consoleModel.prompt)

            actions: [
                Kirigami.Action {
                    text: qsTr("Yes")
                    icon.name: "dialog-ok-apply"

                    onTriggered: page.consoleModel.answer_yes()
                },
                Kirigami.Action {
                    text: qsTr("No")
                    icon.name: "dialog-cancel"

                    onTriggered: page.consoleModel.answer_no()
                }
            ]
        }

        // A password prompt (sudo, run0 on a terminal): typed here, sent to
        // the program and never shown or logged.
        Controls.Pane {
            Layout.fillWidth: true
            visible: page.consoleModel.prompt.length > 0 && page.consoleModel.prompt_password

            contentItem: RowLayout {
                spacing: Kirigami.Units.smallSpacing

                Kirigami.Icon {
                    source: "dialog-password"
                    Layout.preferredWidth: Kirigami.Units.iconSizes.smallMedium
                    Layout.preferredHeight: Kirigami.Units.iconSizes.smallMedium
                }

                Controls.Label {
                    text: page.consoleModel.prompt
                    textFormat: Text.PlainText
                }

                Kirigami.PasswordField {
                    id: passwordField

                    Layout.fillWidth: true

                    onAccepted: sendButton.clicked()
                }

                Controls.Button {
                    id: sendButton

                    text: qsTr("Send")
                    icon.name: "go-next"
                    icon.width: Kirigami.Units.iconSizes.sizeForLabels
                    icon.height: Kirigami.Units.iconSizes.sizeForLabels
                    enabled: passwordField.text.length > 0

                    onClicked: {
                        page.consoleModel.answer_password(passwordField.text);
                        passwordField.text = "";
                    }
                }
            }
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            position: Kirigami.InlineMessage.Position.Header
            visible: page.system.running || page.system.last_result.length > 0
            type: page.system.running ? Kirigami.MessageType.Information : page.system.last_result === Kinds.outcomeOk ? Kirigami.MessageType.Positive : Kirigami.MessageType.Error
            text: page.system.running ? qsTr("Working… output appears below.") : page.system.last_result === Kinds.outcomeOk ? qsTr("Done.") : page.system.can_restore ? qsTr("Failed. Check the output below. You can restore the previous file.") : qsTr("Failed. Check the output below.")

            actions: [
                Kirigami.Action {
                    text: qsTr("Rebuild and switch")
                    icon.name: "system-run"
                    visible: !page.system.running && page.system.last_result === Kinds.outcomeOk && page.updateDone && page.rebuildModes.includes("switch")

                    onTriggered: page.inputsPage.rebuild("switch")
                },
                Kirigami.Action {
                    text: qsTr("Build only")
                    icon.name: "run-build"
                    visible: !page.system.running && page.system.last_result === Kinds.outcomeOk && page.updateDone && page.rebuildModes.includes("build")

                    onTriggered: page.inputsPage.rebuild("build")
                }
            ]
        }
    }
    background: Rectangle {
        color: page.app.consoleColors.loaded ? page.app.consoleColors.background : Kirigami.Theme.backgroundColor
    }

    onRunningChanged: if (page.running)
        list.positionViewAtBeginning()
    onTerminalColumnsChanged: page.sendTerminalSize()
    onTerminalRowsChanged: page.sendTerminalSize()

    FontMetrics {
        id: metrics

        font.family: "monospace"
    }

    Connections {
        function onState_changed(): void {
            if (page.consoleModel.prompt.length > 0 && !page.consoleModel.prompt_password)
                page.rememberQuestion();
        }

        target: page.consoleModel
    }

    ListView {
        id: list

        model: page.consoleModel
        clip: true
        // Newest line at the bottom: the bottom is the anchor, so output printed above nom's live tree leaves it in place.
        verticalLayoutDirection: ListView.BottomToTop

        delegate: Rectangle {
            id: entry

            required property string text
            required property string html
            readonly property string kind: {
                const message = entry.text;
                const lower = message.toLowerCase();
                if (message.startsWith("$ "))
                    return "command";
                if (lower.startsWith("error") || lower.indexOf(" error:") >= 0 || message.startsWith("Failed") || message.startsWith("Not ") || message.startsWith("Cannot ") || message.startsWith("Invalid "))
                    return "error";
                if (message.startsWith("Finished successfully") || message.startsWith("Saved ") || message.startsWith("Restored "))
                    return "ok";
                return "plain";
            }
            readonly property bool hidden: page.isPromptLine(entry.text)

            width: ListView.view.width
            visible: !entry.hidden
            implicitHeight: entry.hidden ? 0 : line.implicitHeight + Kirigami.Units.smallSpacing * 2
            color: "transparent"

            Rectangle {
                visible: entry.kind !== "plain"
                width: Math.round(Kirigami.Units.smallSpacing * 0.75)
                height: parent.height
                color: entry.kind === "command" ? Kirigami.Theme.linkColor : entry.kind === "error" ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.positiveTextColor
            }

            Controls.Label {
                id: line

                leftPadding: Kirigami.Units.largeSpacing
                rightPadding: Kirigami.Units.largeSpacing
                text: entry.html.length > 0 ? entry.html : entry.text
                textFormat: entry.html.length > 0 ? Text.StyledText : Text.PlainText
                font.family: "monospace"
                font.bold: entry.kind === "command"
                wrapMode: Text.WrapAnywhere
                color: entry.kind === "error" ? Kirigami.Theme.negativeTextColor : entry.kind === "command" ? Kirigami.Theme.linkColor : page.app.consoleColors.loaded ? page.app.consoleColors.text : Kirigami.Theme.textColor

                anchors {
                    left: parent.left
                    right: parent.right
                    verticalCenter: parent.verticalCenter
                }
            }
        }

        Kirigami.PlaceholderMessage {
            anchors.centerIn: parent
            width: parent.width - Kirigami.Units.gridUnit * 4
            visible: list.count === 0
            icon.name: "dialog-scripts"
            text: qsTr("No output yet")
            explanation: qsTr("Saving and rebuilding prints its progress here.")
        }
    }

    TextEdit {
        id: clipboardHelper

        visible: false
    }
}
