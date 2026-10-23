pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../components"

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var changes: page.app.changeModel
    readonly property var system: page.app.systemModel
    readonly property var warnings: page.system.warnings
    readonly property var managed: {
        page.changes.revision;

        return page.changes.managed_list();
    }
    readonly property bool checksClear: page.changes.checks_pending === 0 && page.changes.checks_failed === 0
    readonly property bool canApply: page.changes.ready && page.system.target_ready && !page.system.running && page.checksClear
    readonly property var can: page.system.capabilities

    function applyPrimary(): void {
        if (!page.canApply)
            return;

        page.app.startApply("save");
    }

    title: qsTr("Changes")

    actions: [
        Kirigami.Action {
            text: qsTr("Preview file")
            icon.name: "document-preview"
            enabled: page.changes.ready

            onTriggered: page.app.showPreview("")
        },
        Kirigami.Action {
            text: qsTr("Apply")
            icon.name: "dialog-ok-apply"
            enabled: page.canApply
            visible: page.can.apply.includes("switch")
            tooltip: qsTr("Save drakeflake.nix and switch to the new configuration")

            onTriggered: page.app.startApply("switch")
        },
        Kirigami.Action {
            objectName: "reviewApplyAction"
            text: qsTr("Review and apply…")
            icon.name: "document-preview-archive"
            enabled: page.canApply
            visible: page.can.review.includes("switch")
            tooltip: qsTr("Show the generated file first, then save, build and ask before switching")

            onTriggered: page.app.showPreview("switch")
        },
        Kirigami.Action {
            objectName: "reviewBootAction"
            text: qsTr("Review and apply on next boot…")
            icon.name: "system-reboot"
            enabled: page.canApply
            visible: page.can.review.includes("boot")
            displayHint: Kirigami.DisplayHint.AlwaysHide

            onTriggered: page.app.showPreview("boot")
        },
        Kirigami.Action {
            text: qsTr("Save only")
            icon.name: "document-save"
            enabled: page.canApply
            displayHint: page.can.native ? Kirigami.DisplayHint.AlwaysHide : Kirigami.DisplayHint.NoPreference
            tooltip: qsTr("Write drakeflake.nix without rebuilding")

            onTriggered: page.app.startApply("save")
        },
        Kirigami.Action {
            text: qsTr("Save and test build")
            icon.name: "run-build"
            enabled: page.canApply
            visible: page.can.apply.includes("build")
            displayHint: Kirigami.DisplayHint.AlwaysHide
            tooltip: qsTr("Write the file and build the system without activating it")

            onTriggered: page.app.startApply("build")
        },
        Kirigami.Action {
            text: qsTr("Apply until reboot (test)")
            icon.name: "media-playback-start"
            enabled: page.canApply
            visible: page.can.apply.includes("test")
            displayHint: Kirigami.DisplayHint.AlwaysHide

            onTriggered: page.app.startApply("test")
        },
        Kirigami.Action {
            text: qsTr("Apply on next boot")
            icon.name: "system-reboot"
            enabled: page.canApply
            visible: page.can.apply.includes("boot")
            displayHint: Kirigami.DisplayHint.AlwaysHide

            onTriggered: page.app.startApply("boot")
        },
        Kirigami.Action {
            text: qsTr("Discard all")
            icon.name: "edit-clear-all"
            enabled: page.changes.count > 0
            displayHint: Kirigami.DisplayHint.AlwaysHide

            onTriggered: page.changes.clear()
        }
    ]

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        Repeater {
            model: page.warnings

            delegate: Kirigami.InlineMessage {
                id: warning

                required property var modelData

                Layout.fillWidth: true
                visible: true
                type: Kirigami.MessageType.Warning
                text: warning.modelData.text
                // Stable array identity: a fresh array per refresh would retrigger the toolbar menu incubation.
                actions: [trackAction]

                Kirigami.Action {
                    id: trackAction

                    visible: warning.modelData.id === "untracked"
                    text: qsTr("Track in git")
                    icon.name: "vcs-normal"

                    onTriggered: page.system.track_in_git()
                }
            }
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: !page.checksClear
            type: page.changes.checks_failed > 0 ? Kirigami.MessageType.Error : Kirigami.MessageType.Information
            text: page.changes.checks_failed > 0 ? qsTr("%1 staged value(s) do not match their option type. Fix or unstage them to apply.").arg(page.changes.checks_failed) : qsTr("Checking staged values against your configuration's option types…")
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: page.system.target_ready && !page.can.native
            type: Kirigami.MessageType.Information
            text: page.system.kind === "darwin" ? qsTr("This is a nix-darwin host. On this Linux machine it can only be saved. Build and switch to it on the Mac.") : qsTr("This is a NixOS host. On this Mac it can only be saved. Build and switch to it on the NixOS machine.")
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: page.changes.error.length > 0
            type: Kirigami.MessageType.Error
            text: page.changes.error
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: page.system.target_error.length > 0
            type: Kirigami.MessageType.Error
            text: page.system.target_error

            actions: [
                Kirigami.Action {
                    text: qsTr("Settings")
                    icon.name: "settings-configure"

                    onTriggered: page.app.showSection("settings")
                }
            ]
        }

        Controls.Label {
            Layout.fillWidth: true
            visible: page.system.target_ready
            wrapMode: Text.WrapAnywhere
            opacity: 0.75
            text: qsTr("Target: %1#%2").arg(page.system.flake).arg(page.system.host) + (page.system.managed_path.length > 0 ? "\n" + qsTr("File: %1").arg(page.system.managed_path) : "")
        }

        SectionHeading {
            text: page.changes.count > 0 ? qsTr("Pending changes (%1)").arg(page.changes.count) : qsTr("Pending changes")
        }

        Kirigami.PlaceholderMessage {
            Layout.fillWidth: true
            visible: page.changes.count === 0
            icon.name: "dialog-ok-apply"
            text: qsTr("Nothing staged")
            explanation: qsTr("Change options or add packages. They collect here until you apply them.")
        }

        SettingText {
            id: settingText
        }

        Repeater {
            model: page.changes

            delegate: Controls.ItemDelegate {
                id: row

                required property string key
                required property string kind
                required property string name
                required property string change_type
                required property string before
                required property string after
                required property string check
                required property string check_message
                required property string priority
                required property string mode
                required property string host
                readonly property string settings: settingText.summary(row.priority, row.mode, row.host)

                Layout.fillWidth: true

                contentItem: RowLayout {
                    spacing: Kirigami.Units.largeSpacing

                    Kirigami.Icon {
                        source: {
                            switch (row.change_type) {
                            case "add":
                                return "list-add";
                            case "remove":
                                return "list-remove";
                            case "reset":
                                return "edit-delete";
                            default:
                                return "document-edit";
                            }
                        }
                        Layout.preferredWidth: Kirigami.Units.iconSizes.smallMedium
                        Layout.preferredHeight: Kirigami.Units.iconSizes.smallMedium
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0

                        Controls.Label {
                            Layout.fillWidth: true
                            textFormat: Text.PlainText
                            text: row.name
                            font.family: "monospace"
                            font.bold: true
                            wrapMode: Text.WrapAnywhere
                        }

                        Controls.Label {
                            Layout.fillWidth: true
                            opacity: 0.75
                            elide: Text.ElideRight
                            text: {
                                switch (row.change_type) {
                                case "add":
                                    return qsTr("Add package");
                                case "remove":
                                    return qsTr("Remove package");
                                case "reset":
                                    return qsTr("Remove from DrakeFlake file (was %1)").arg(row.before);
                                default:
                                    return row.before.length > 0 ? row.before + "  →  " + row.after : qsTr("Set to %1").arg(row.after);
                                }
                            }
                        }

                        Controls.Label {
                            Layout.fillWidth: true
                            visible: row.settings.length > 0
                            textFormat: Text.PlainText
                            text: row.settings
                            elide: Text.ElideRight
                            font: Kirigami.Theme.smallFont
                            color: Kirigami.Theme.linkColor
                        }

                        CheckStatus {
                            Layout.fillWidth: true
                            visible: row.check.length > 0
                            check: row.check
                            message: row.check_message
                        }
                    }

                    Controls.ToolButton {
                        id: unstageButton

                        icon.name: "edit-undo"
                        icon.width: Kirigami.Units.iconSizes.sizeForLabels
                        icon.height: Kirigami.Units.iconSizes.sizeForLabels
                        text: qsTr("Unstage")
                        display: Controls.AbstractButton.IconOnly
                        Controls.ToolTip.visible: unstageButton.hovered
                        Controls.ToolTip.text: unstageButton.text

                        onClicked: page.changes.unstage(row.key)
                    }
                }

                onClicked: if (row.kind === "option")
                    page.app.showOption(row.name)
            }
        }

        SectionHeading {
            visible: page.managed.length > 0
            text: qsTr("Managed by DrakeFlake (%1)").arg(page.managed.length)
        }

        Controls.Label {
            Layout.fillWidth: true
            visible: page.managed.length > 0
            wrapMode: Text.Wrap
            opacity: 0.7
            font: Kirigami.Theme.smallFont
            text: qsTr("Already saved in drakeflake.nix. Removing an entry hands the setting back to the rest of your configuration.")
        }

        Repeater {
            model: page.managed

            delegate: RowLayout {
                id: entry

                required property var modelData

                Layout.fillWidth: true
                spacing: Kirigami.Units.largeSpacing

                Kirigami.Icon {
                    source: entry.modelData.kind === "package" ? "package-x-generic" : "configure"
                    Layout.preferredWidth: Kirigami.Units.iconSizes.small
                    Layout.preferredHeight: Kirigami.Units.iconSizes.small
                }

                Controls.Label {
                    Layout.fillWidth: true
                    textFormat: Text.PlainText
                    text: entry.modelData.name + (entry.modelData.value ? " = " + entry.modelData.value : "")
                    font.family: "monospace"
                    elide: Text.ElideRight
                }

                Controls.ToolButton {
                    id: removeButton

                    icon.name: "edit-delete"
                    icon.width: Kirigami.Units.iconSizes.sizeForLabels
                    icon.height: Kirigami.Units.iconSizes.sizeForLabels
                    text: qsTr("Remove")
                    display: Controls.AbstractButton.IconOnly
                    Controls.ToolTip.visible: removeButton.hovered
                    Controls.ToolTip.text: qsTr("Stage removal")

                    onClicked: {
                        if (entry.modelData.kind === "package")
                            page.changes.stage_package(entry.modelData.name, false);
                        else
                            page.changes.stage_reset(entry.modelData.name);
                    }
                }
            }
        }
    }
}
