import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../main/helpers.js" as Helpers
import "../../main/kinds.js" as Kinds

ColumnLayout {
    id: actions

    required property var app
    required property bool editable
    required property bool expressionMode
    required property string kind
    required property string validation
    required property string stageError
    required property bool dirty
    required property string stagedState
    required property string checkStatus
    required property string checkMessage

    signal stageRequested
    signal revertRequested
    signal unstageRequested
    signal removeRequested

    // An empty group would still add spacing in the page's column.
    visible: actions.editable || actions.checkStatus.length > 0
    spacing: Kirigami.Units.largeSpacing

    Controls.Label {
        id: hint

        Layout.fillWidth: true
        visible: actions.editable && hint.text.length > 0
        wrapMode: Text.Wrap
        font: Kirigami.Theme.smallFont
        opacity: 0.75
        text: {
            if (actions.expressionMode)
                return qsTr("Written as typed, in parentheses. The names config, lib and pkgs are in scope. It is parsed and type-checked before saving.");
            if (actions.kind === "raw")
                return qsTr("Entered as a Nix expression. It is syntax-checked before saving.");
            return "";
        }
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: actions.editable && (actions.validation.length > 0 || actions.stageError.length > 0)
        type: Kirigami.MessageType.Error
        text: Helpers.richText(actions.stageError.length > 0 ? actions.stageError : actions.validation)
    }

    Flow {
        Layout.fillWidth: true
        visible: actions.editable
        spacing: Kirigami.Units.smallSpacing

        Controls.Button {
            text: actions.stagedState === "set" ? qsTr("Update staged change") : qsTr("Stage change")
            icon.name: "dialog-ok-apply"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels
            highlighted: true
            enabled: actions.dirty && actions.validation.length === 0

            onClicked: actions.stageRequested()
        }

        Controls.Button {
            text: qsTr("Revert")
            icon.name: "edit-undo"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels
            enabled: actions.dirty

            onClicked: actions.revertRequested()
        }

        Controls.Button {
            visible: actions.stagedState === "set" || actions.stagedState === "reset"
            text: qsTr("Unstage")
            icon.name: "edit-clear"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels

            onClicked: actions.unstageRequested()
        }

        Controls.Button {
            visible: actions.stagedState === "managed"
            text: qsTr("Remove from DrakeFlake file")
            icon.name: "edit-delete"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels

            onClicked: actions.removeRequested()
        }
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: actions.checkStatus.length > 0
        type: actions.checkStatus === Kinds.checkOk ? Kirigami.MessageType.Positive : actions.checkStatus === Kinds.checkInvalid || actions.checkStatus === Kinds.checkFailed ? Kirigami.MessageType.Error : Kirigami.MessageType.Information
        text: Helpers.richText(actions.checkMessage)
    }
}
