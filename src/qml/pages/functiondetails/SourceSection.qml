pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers

ColumnLayout {
    id: section

    required property var app
    required property var info
    readonly property var lambda: section.info.lambda || null
    readonly property var attr: section.info.attr || null
    readonly property bool attrElsewhere: section.attr !== null && (section.lambda === null || section.attr.file !== section.lambda.file || Math.abs(section.attr.line - section.lambda.line) > 3)

    function place(position: var): string {
        const label = position.file + ":" + position.line;
        return position.url.length > 0 ? Helpers.link(position.url, label) : Helpers.escapeHtml(label);
    }

    visible: section.lambda !== null || section.attr !== null || (section.info.code || "").length > 0
    spacing: Kirigami.Units.largeSpacing

    SectionHeading {
        text: qsTr("Source")
    }

    Controls.Label {
        Layout.fillWidth: true
        visible: section.lambda !== null
        text: section.lambda ? qsTr("Defined in %1").arg(section.place(section.lambda)) : ""
        textFormat: Text.StyledText
        wrapMode: Text.WrapAnywhere

        onLinkActivated: link => section.app.openExternal(link)
    }

    Controls.Label {
        Layout.fillWidth: true
        visible: section.attrElsewhere
        text: section.attr ? qsTr("Set as an attribute in %1").arg(section.place(section.attr)) : ""
        textFormat: Text.StyledText
        wrapMode: Text.WrapAnywhere

        onLinkActivated: link => section.app.openExternal(link)
    }

    CodeBlock {
        objectName: "functionCode"
        Layout.fillWidth: true
        visible: (section.info.code || "").length > 0
        text: section.info.code || ""
        themeName: section.app.highlightTheme
    }
}
