import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"

ColumnLayout {
    id: header

    required property var app
    required property string optionName
    required property var info
    required property string stagedState
    required property var stagedSettings

    function copyName(): void {
        nameField.selectAll();
        nameField.copy();
        nameField.deselect();
    }

    spacing: Kirigami.Units.largeSpacing

    SettingText {
        id: settingText
    }

    TextEdit {
        id: nameField

        Layout.fillWidth: true
        text: header.optionName
        readOnly: true
        selectByMouse: true
        wrapMode: TextEdit.WrapAnywhere
        textFormat: TextEdit.PlainText
        color: Kirigami.Theme.textColor
        selectionColor: Kirigami.Theme.highlightColor
        selectedTextColor: Kirigami.Theme.highlightedTextColor

        font {
            family: "monospace"
            bold: true
            pointSize: Kirigami.Theme.defaultFont.pointSize * 1.15
        }
    }

    Flow {
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing

        Tag {
            text: header.info.type || ""
        }

        Tag {
            visible: header.info.readOnly === true
            text: qsTr("read-only")
        }

        Tag {
            visible: header.info.templated === true
            text: qsTr("template")
        }

        Tag {
            visible: (header.info.userFiles || []).length > 0
            text: qsTr("set in your configuration")
            tone: "positive"
        }

        Tag {
            visible: header.stagedState === "managed"
            text: qsTr("managed by DrakeFlake")
            tone: "positive"
        }

        Tag {
            visible: header.stagedState === "set"
            text: qsTr("change staged")
            tone: "warning"
        }

        Tag {
            id: settingsTag

            visible: settingsTag.text.length > 0
            text: header.stagedSettings.mode ? settingText.summary(header.stagedSettings.priority, header.stagedSettings.mode, header.stagedSettings.host) : ""
            tone: "info"
        }

        Tag {
            visible: header.stagedState === "reset"
            text: qsTr("removal staged")
            tone: "warning"
        }
    }

    Controls.Label {
        id: description

        Layout.fillWidth: true
        text: header.app.docText(header.info.description || "")
        textFormat: Text.MarkdownText
        wrapMode: Text.Wrap

        onLinkActivated: link => header.app.openExternal(link)

        HoverHandler {
            cursorShape: description.hoveredLink ? Qt.PointingHandCursor : Qt.ArrowCursor
        }
    }
}
