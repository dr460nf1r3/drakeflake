pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers

Controls.Control {
    id: row

    required property var modelData
    required property var app
    required property bool checking
    required property bool busy
    required property bool open
    readonly property var check: row.modelData.check || null
    readonly property string inputState: {
        if (row.modelData.follows)
            return "follows";

        if (!row.check)
            return row.checking ? "checking" : "unknown";

        return row.check.state;
    }
    readonly property var nested: row.modelData.nested || []

    signal toggled
    signal updateRequested

    padding: Kirigami.Units.largeSpacing

    background: Item {
        TapHandler {
            enabled: row.nested.length > 0

            onTapped: row.toggled()
        }

        Kirigami.Separator {
            anchors {
                left: parent.left
                right: parent.right
                bottom: parent.bottom
            }
        }
    }
    contentItem: RowLayout {
        spacing: Kirigami.Units.largeSpacing

        ColumnLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing

            Flow {
                Layout.fillWidth: true
                spacing: Kirigami.Units.smallSpacing

                Controls.Label {
                    textFormat: Text.PlainText
                    text: row.modelData.name

                    font {
                        family: "monospace"
                        bold: true
                    }
                }

                Tag {
                    visible: row.modelData.kind.length > 0 && !row.modelData.follows
                    text: row.modelData.kind
                }

                Tag {
                    visible: row.inputState === "available"
                    text: qsTr("update available")
                    tone: "warning"
                }

                Tag {
                    visible: row.inputState === "up-to-date"
                    text: qsTr("up to date")
                    tone: "positive"
                }

                Tag {
                    visible: row.inputState === "failed"
                    text: qsTr("check failed")
                    tone: "negative"
                }

                Tag {
                    visible: row.modelData.pinned
                    text: qsTr("pinned")
                }
            }

            Controls.Label {
                Layout.fillWidth: true
                visible: !row.modelData.follows
                text: row.modelData.web_url ? Helpers.link(row.modelData.web_url, row.modelData.source) : Helpers.escapeHtml(row.modelData.source)
                textFormat: Text.StyledText
                font.family: "monospace"
                wrapMode: Text.WrapAnywhere

                onLinkActivated: link => row.app.openExternal(link)
            }

            Controls.Label {
                Layout.fillWidth: true
                visible: !!row.modelData.follows
                opacity: 0.75
                text: qsTr("Follows the input “%1”: it is locked with that input.").arg(row.modelData.follows || "")
                textFormat: Text.PlainText
                wrapMode: Text.Wrap
            }

            Controls.Label {
                Layout.fillWidth: true
                visible: !!row.modelData.rev
                opacity: 0.75
                wrapMode: Text.Wrap
                textFormat: Text.StyledText
                text: {
                    const rev = Helpers.shortRev(row.modelData.rev || "");
                    const linked = row.modelData.commit_url ? Helpers.link(row.modelData.commit_url, rev) : Helpers.escapeHtml(rev);
                    const date = Helpers.dateWithAge(row.modelData.last_modified);
                    return date.length > 0 ? qsTr("Locked at %1 · %2").arg(linked).arg(date) : qsTr("Locked at %1").arg(linked);
                }

                onLinkActivated: link => row.app.openExternal(link)
            }

            Controls.Label {
                Layout.fillWidth: true
                visible: row.inputState === "available"
                wrapMode: Text.Wrap
                color: Kirigami.Theme.neutralTextColor
                textFormat: Text.StyledText
                text: {
                    const latest = qsTr("Newest upstream: %1").arg(Helpers.escapeHtml(Helpers.shortRev(row.check ? row.check.rev || "" : "")));
                    const compare = row.check ? row.check.compare_url : "";
                    return compare ? latest + " · " + Helpers.link(compare, qsTr("see all changes")) : latest;
                }

                onLinkActivated: link => row.app.openExternal(link)
            }

            Controls.Label {
                Layout.fillWidth: true
                visible: row.inputState === "not-checkable" || row.inputState === "failed"
                opacity: row.inputState === "failed" ? 1 : 0.75
                color: row.inputState === "failed" ? Kirigami.Theme.negativeTextColor : Kirigami.Theme.textColor
                wrapMode: Text.Wrap
                textFormat: Text.PlainText
                text: {
                    if (!row.check)
                        return "";

                    return row.inputState === "failed" ? qsTr("Could not check: %1").arg(row.check.error || "") : qsTr("Not checked: %1").arg(row.check.reason || "");
                }
            }

            Kirigami.LinkButton {
                visible: row.nested.length > 0
                horizontalAlignment: Text.AlignLeft

                action: Kirigami.Action {
                    text: row.nested.length === 1 ? qsTr("1 input of its own") : qsTr("%1 inputs of its own").arg(row.nested.length)

                    onTriggered: row.toggled()
                }
            }

            Repeater {
                model: row.open ? row.nested : []

                delegate: NestedInput {
                    Layout.fillWidth: true
                    Layout.leftMargin: Kirigami.Units.gridUnit
                    parentName: row.modelData.name
                }
            }
        }

        Controls.BusyIndicator {
            id: checkingIndicator

            Layout.preferredWidth: Kirigami.Units.iconSizes.sizeForLabels
            Layout.preferredHeight: Kirigami.Units.iconSizes.sizeForLabels
            visible: row.inputState === "checking"
            running: checkingIndicator.visible
        }

        Controls.Button {
            objectName: "updateButton"
            visible: !row.modelData.follows && !row.modelData.pinned
            text: qsTr("Update")
            icon.name: "update-none"
            icon.width: Kirigami.Units.iconSizes.sizeForLabels
            icon.height: Kirigami.Units.iconSizes.sizeForLabels
            highlighted: row.inputState === "available"
            enabled: !row.busy

            onClicked: row.updateRequested()
        }
    }
}
