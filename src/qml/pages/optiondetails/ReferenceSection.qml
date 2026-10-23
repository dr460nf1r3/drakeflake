pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers

ColumnLayout {
    id: reference

    required property var app
    required property var info
    readonly property bool hasDefault: reference.info.default !== undefined && reference.info.default !== null
    readonly property bool hasExample: reference.info.example !== undefined && reference.info.example !== null
    readonly property var declarations: reference.info.declarations || []

    // An empty group would still add spacing in the page's column.
    visible: reference.hasDefault || reference.hasExample || reference.declarations.length > 0
    spacing: Kirigami.Units.largeSpacing

    SectionHeading {
        visible: reference.hasDefault
        text: qsTr("Default")
    }

    CodeBlock {
        Layout.fillWidth: true
        themeName: reference.app.highlightTheme
        visible: reference.hasDefault
        text: reference.info.default || ""
    }

    SectionHeading {
        visible: reference.hasExample
        text: qsTr("Example")
    }

    CodeBlock {
        Layout.fillWidth: true
        themeName: reference.app.highlightTheme
        visible: reference.hasExample
        text: reference.info.example || ""
    }

    SectionHeading {
        visible: reference.declarations.length > 0
        text: qsTr("Declared in")
    }

    Repeater {
        model: reference.declarations

        delegate: Controls.Label {
            id: declaration

            required property string modelData
            readonly property string url: declaration.modelData.startsWith("nixos/") || declaration.modelData.startsWith("pkgs/") ? "https://github.com/NixOS/nixpkgs/blob/master/" + declaration.modelData : ""

            Layout.fillWidth: true
            text: declaration.url.length > 0 ? Helpers.link(declaration.url, declaration.modelData) : declaration.modelData
            textFormat: declaration.url.length > 0 ? Text.StyledText : Text.PlainText
            wrapMode: Text.WrapAnywhere
            font.family: "monospace"

            onLinkActivated: link => reference.app.openExternal(link)

            HoverHandler {
                cursorShape: declaration.hoveredLink ? Qt.PointingHandCursor : Qt.ArrowCursor
            }
        }
    }
}
