pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

// Button opening a filterable list because ComboBox popups scroll badly and paint outside themselves.
Controls.Button {
    id: picker

    property var options: []
    property string currentValue: ""
    property string filterPlaceholder: qsTr("Filter…")
    readonly property string currentLabel: {
        const found = picker.options.find(option => option.value === picker.currentValue);
        return found ? found.label : "";
    }
    property alias filterText: filter.text
    readonly property bool popupOpen: popup.opened

    signal picked(string value)

    function open(): void {
        popup.open();
    }

    function close(): void {
        popup.close();
    }

    function choose(value: string): void {
        popup.close();

        if (value !== picker.currentValue)
            picker.picked(value);
    }

    text: picker.currentLabel
    icon.name: "arrow-down"
    icon.width: Kirigami.Units.iconSizes.sizeForLabels
    icon.height: Kirigami.Units.iconSizes.sizeForLabels
    LayoutMirroring.enabled: true
    LayoutMirroring.childrenInherit: false

    onClicked: popup.open()

    Controls.Popup {
        id: popup

        objectName: "popup"
        popupType: Controls.Popup.Item
        y: picker.height
        width: Math.max(picker.width, Kirigami.Units.gridUnit * 18)
        height: Kirigami.Units.gridUnit * 24
        padding: Kirigami.Units.smallSpacing
        focus: true

        // The application palette stays light where the theme is dark, so the surface is set here.
        background: Rectangle {
            color: Kirigami.Theme.backgroundColor
            radius: Kirigami.Units.cornerRadius
            border.width: 1
            border.color: Qt.alpha(Kirigami.Theme.textColor, 0.25)
        }
        contentItem: ColumnLayout {
            spacing: Kirigami.Units.smallSpacing

            Kirigami.SearchField {
                id: filter

                focusSequences: []
                objectName: "filter"
                Layout.fillWidth: true
                placeholderText: picker.filterPlaceholder
                // Only Enter picks: auto-accept would pick while typing.
                autoAccept: false

                onAccepted: {
                    if (list.count > 0)
                        picker.choose(list.model[0].value);
                }

                FieldMenu {
                    target: filter
                }
            }

            ListView {
                id: list

                objectName: "list"
                Layout.fillWidth: true
                Layout.fillHeight: true
                clip: true
                model: {
                    const needle = filter.text.trim().toLowerCase();
                    return needle.length === 0 ? picker.options : picker.options.filter(option => option.label.toLowerCase().indexOf(needle) >= 0);
                }

                Controls.ScrollBar.vertical: Controls.ScrollBar {
                }
                delegate: Controls.ItemDelegate {
                    id: choice

                    required property var modelData

                    objectName: "choice"
                    width: ListView.view.width
                    text: choice.modelData.label
                    highlighted: choice.modelData.value === picker.currentValue

                    onClicked: picker.choose(choice.modelData.value)
                }

                Kirigami.PlaceholderMessage {
                    objectName: "emptyHint"
                    anchors.centerIn: parent
                    width: parent.width - Kirigami.Units.gridUnit * 2
                    visible: list.count === 0
                    text: qsTr("No match")
                }
            }
        }

        onOpened: {
            filter.text = "";
            filter.forceActiveFocus();
            list.positionViewAtIndex(Math.max(0, list.model.findIndex(option => option.value === picker.currentValue)), ListView.Center);
        }
    }
}
