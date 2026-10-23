pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

Flow {
    id: filters

    required property var options
    property string area: ""
    property bool onlyUser: false
    property bool showTemplated: false
    property var areas: []
    property int loadedSize: -1

    signal areaPicked(string area)
    signal onlyUserToggled(bool onlyUser)
    signal showTemplatedToggled(bool showTemplated)

    function loadAreas(): void {
        filters.loadedSize = filters.options.size;
        filters.areas = filters.options.namespaces();
    }

    spacing: Kirigami.Units.smallSpacing

    Component.onCompleted: filters.loadAreas()

    Connections {
        function onState_changed(): void {
            if (filters.options.size !== filters.loadedSize)
                Qt.callLater(filters.loadAreas);
        }

        target: filters.options
    }

    FilterPicker {
        id: areaPicker

        objectName: "areaPicker"
        options: [
            {
                value: "",
                label: qsTr("All areas")
            }
        ].concat(filters.areas.map(entry => ({
                    value: entry.name,
                    label: qsTr("%1 (%2)").arg(entry.name).arg(entry.count)
                })))
        currentValue: filters.area
        filterPlaceholder: qsTr("Filter areas…")

        onPicked: value => filters.areaPicked(value)
    }

    Item {
        implicitWidth: userBox.implicitWidth
        implicitHeight: areaPicker.implicitHeight

        Controls.CheckBox {
            id: userBox

            objectName: "userBox"
            anchors.verticalCenter: parent.verticalCenter
            text: qsTr("Set in my configuration")
            checked: filters.onlyUser
            enabled: filters.options.values_ready || userBox.checked
            Controls.ToolTip.visible: userBox.hovered
            Controls.ToolTip.text: filters.options.values_ready ? qsTr("Only options your own files define") : qsTr("Available once current values are evaluated")

            onToggled: filters.onlyUserToggled(userBox.checked)
        }
    }

    Item {
        implicitWidth: templateBox.implicitWidth
        implicitHeight: areaPicker.implicitHeight

        Controls.CheckBox {
            id: templateBox

            objectName: "templateBox"
            anchors.verticalCenter: parent.verticalCenter
            text: qsTr("Include <name> templates")
            checked: filters.showTemplated

            onToggled: filters.showTemplatedToggled(templateBox.checked)
        }
    }
}
