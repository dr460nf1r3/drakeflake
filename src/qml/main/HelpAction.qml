import org.kde.kirigami as Kirigami

Kirigami.Action {
    id: help

    required property var app
    required property string section

    objectName: "helpAction"
    text: qsTr("Help")
    icon.name: "help-contents"
    displayHint: Kirigami.DisplayHint.IconOnly
    tooltip: qsTr("Open the documentation about this page")

    onTriggered: help.app.showHelp(help.section)
}
