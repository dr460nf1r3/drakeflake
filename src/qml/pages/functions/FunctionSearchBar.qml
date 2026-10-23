pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers
import "names.js" as Names

Controls.Control {
    id: bar

    property bool clear: false
    required property var functions
    required property string query
    readonly property var kindOptions: Names.kindOptions(bar.functions.kinds())
    readonly property bool filtered: bar.functions.namespace !== "" || bar.functions.takes !== "" || bar.functions.returns !== ""

    signal queryEdited(string text)

    function syncKinds(): void {
        takesBox.currentIndex = Math.max(0, takesBox.indexOfValue(bar.functions.takes));
        returnsBox.currentIndex = Math.max(0, returnsBox.indexOfValue(bar.functions.returns));
    }

    function focusSearch(): void {
        field.forceActiveFocus();
    }

    padding: Kirigami.Units.largeSpacing

    background: SearchHeaderBackground {
        clear: bar.clear
    }
    contentItem: ColumnLayout {
        spacing: Kirigami.Units.smallSpacing

        DebouncedSearchField {
            id: field

            objectName: "functionSearch"
            Layout.fillWidth: true
            placeholderText: bar.functions.size > 0 ? qsTr("Search %1 functions by name, description or type…").arg(Helpers.count(bar.functions.size)) : qsTr("Search functions…")
            text: bar.query

            onQueryEdited: query => bar.queryEdited(query)
            onQueryChanged: query => bar.functions.search(query)
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.smallSpacing
            visible: bar.functions.state === "ready"

            FilterPicker {
                objectName: "functionNamespaceBox"
                Layout.fillWidth: true
                Layout.minimumWidth: Kirigami.Units.gridUnit * 4
                Layout.preferredWidth: 1
                options: [
                    {
                        value: "",
                        label: qsTr("All namespaces")
                    }
                ].concat(bar.functions.namespaces.map(namespace => ({
                            value: namespace.name,
                            label: qsTr("%1 (%2)").arg(namespace.name).arg(namespace.count)
                        })))
                currentValue: bar.functions.namespace
                filterPlaceholder: qsTr("Filter namespaces…")

                onPicked: value => bar.functions.set_namespace(value)
            }

            Controls.ComboBox {
                id: takesBox

                objectName: "functionTakesBox"
                Layout.fillWidth: true
                Layout.minimumWidth: Kirigami.Units.gridUnit * 4
                Layout.preferredWidth: 1
                model: bar.kindOptions
                textRole: "text"
                valueRole: "value"
                displayText: qsTr("Takes: %1").arg(takesBox.currentText)
                Controls.ToolTip.visible: takesBox.hovered && !takesBox.popup.visible
                Controls.ToolTip.text: qsTr("The kind of the first argument, from the documented type")

                Component.onCompleted: bar.syncKinds()
                onActivated: bar.functions.set_takes(takesBox.currentValue)
            }

            Controls.ComboBox {
                id: returnsBox

                objectName: "functionReturnsBox"
                Layout.fillWidth: true
                Layout.minimumWidth: Kirigami.Units.gridUnit * 4
                Layout.preferredWidth: 1
                model: bar.kindOptions
                textRole: "text"
                valueRole: "value"
                displayText: qsTr("Returns: %1").arg(returnsBox.currentText)
                Controls.ToolTip.visible: returnsBox.hovered && !returnsBox.popup.visible
                Controls.ToolTip.text: qsTr("The kind of the result, from the documented type")

                Component.onCompleted: bar.syncKinds()
                onActivated: bar.functions.set_returns(returnsBox.currentValue)
            }

            Controls.ToolButton {
                id: clearFilters

                visible: bar.filtered
                display: Controls.AbstractButton.IconOnly
                text: qsTr("Clear filters")
                icon.name: "edit-clear-all"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels
                Accessible.name: clearFilters.text
                Controls.ToolTip.visible: clearFilters.hovered
                Controls.ToolTip.text: clearFilters.text

                onClicked: {
                    bar.functions.set_namespace("");
                    bar.functions.set_takes("");
                    bar.functions.set_returns("");
                }
            }
        }

        Controls.Label {
            Layout.fillWidth: true
            visible: bar.functions.browsing
            opacity: 0.7
            font: Kirigami.Theme.smallFont
            text: qsTr("%1 matches").arg(Helpers.count(bar.functions.total))
        }
    }

    // Filters and queries set from elsewhere (a namespace card, the
    // Overview). Text still waiting for the debounce is not overwritten.
    Connections {
        function onState_changed(): void {
            if (takesBox.currentValue !== bar.functions.takes || returnsBox.currentValue !== bar.functions.returns)
                bar.syncKinds();
            if (!field.pending && field.text !== bar.functions.query) {
                field.text = bar.functions.query;
                bar.queryEdited(field.text);
            }
        }

        target: bar.functions
    }
}
