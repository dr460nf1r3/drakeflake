import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers

Flow {
    id: filters

    required property var packages
    required property var setList
    required property bool onlyInstalled
    readonly property real rowHeight: Math.max(setBox.visible ? setBox.implicitHeight : 0, installedOnly.implicitHeight)

    signal onlyInstalledToggled(bool checked)

    spacing: Kirigami.Units.smallSpacing

    FilterPicker {
        id: setBox

        objectName: "packageSetBox"
        width: Math.min(setBox.implicitWidth, Kirigami.Units.gridUnit * 16, filters.width)
        visible: filters.setList.length > 0
        options: [
            {
                value: "",
                label: qsTr("Top-level packages")
            }
        ].concat(filters.setList.map(name => ({
                    value: name,
                    label: name
                })))
        currentValue: filters.packages.package_set
        filterPlaceholder: qsTr("Filter package sets…")
        Controls.ToolTip.visible: setBox.hovered && !setBox.down
        Controls.ToolTip.text: qsTr("Package sets such as python313Packages or haskellPackages")

        onPicked: value => filters.packages.set_package_set(value)
    }

    Item {
        width: Kirigami.Units.iconSizes.smallMedium
        height: filters.rowHeight
        visible: filters.packages.set_loading

        Controls.BusyIndicator {
            width: Kirigami.Units.iconSizes.smallMedium
            height: Kirigami.Units.iconSizes.smallMedium
            anchors.centerIn: parent
            running: filters.packages.set_loading
        }
    }

    Controls.CheckBox {
        id: installedOnly

        height: filters.rowHeight
        text: qsTr("Installed only")
        checked: filters.onlyInstalled

        onToggled: filters.onlyInstalledToggled(installedOnly.checked)
    }

    Controls.CheckBox {
        id: commandMode

        height: filters.rowHeight
        text: qsTr("Search by command")
        checked: filters.packages.command_mode
        Controls.ToolTip.visible: commandMode.hovered
        Controls.ToolTip.text: qsTr("Find the package that provides a program, e.g. rg -> ripgrep")

        onToggled: filters.packages.set_command_mode(commandMode.checked)
    }

    Controls.Label {
        height: filters.rowHeight
        verticalAlignment: Text.AlignVCenter
        visible: filters.packages.size > 0
        opacity: 0.7
        font: Kirigami.Theme.smallFont
        text: qsTr("%1 matches · %2 installed").arg(Helpers.count(filters.packages.total)).arg(filters.packages.installed_count)
    }
}
