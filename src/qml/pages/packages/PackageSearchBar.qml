import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers
import "../../main/kinds.js" as Kinds

Controls.Control {
    id: bar

    required property var app
    required property var packages
    required property string query
    required property bool onlyInstalled
    required property var setList
    required property string stageError

    signal queryEdited(string text)
    signal onlyInstalledToggled(bool checked)
    signal stageErrorDismissed

    function focusSearch(): void {
        packageSearch.forceActiveFocus();
    }

    padding: Kirigami.Units.largeSpacing

    background: SearchHeaderBackground {
        clear: bar.app.vibrancy
    }
    contentItem: ColumnLayout {
        spacing: Kirigami.Units.smallSpacing

        DebouncedSearchField {
            id: packageSearch

            Layout.fillWidth: true
            placeholderText: {
                if (bar.packages.command_mode)
                    return qsTr("Which package has the program… (e.g. rg)");

                const count = Helpers.count(bar.packages.size);
                const set = bar.packages.package_set;

                if (bar.packages.size === 0)
                    return set.length > 0 ? qsTr("Search %1…").arg(set) : qsTr("Search packages…");

                return set.length > 0 ? qsTr("Search %1 packages in %2…").arg(count).arg(set) : qsTr("Search %1 packages…").arg(count);
            }
            text: bar.query

            onQueryEdited: query => bar.queryEdited(query)
            onQueryChanged: query => bar.packages.search(query)
        }

        PackageFilters {
            Layout.fillWidth: true
            packages: bar.packages
            setList: bar.setList
            onlyInstalled: bar.onlyInstalled

            onOnlyInstalledToggled: checked => bar.onlyInstalledToggled(checked)
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: bar.packages.set_error.length > 0
            type: Kirigami.MessageType.Warning
            text: Helpers.richText(qsTr("Could not index %1: %2").arg(bar.packages.package_set).arg(bar.packages.set_error))
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: bar.packages.command_mode && bar.packages.command_state === Kinds.commandMissingIndex
            type: Kirigami.MessageType.Information
            text: qsTr("Searching by command needs the command index (nix-index, updated weekly, about 100 MB).")

            actions: [
                Kirigami.Action {
                    text: qsTr("Download")
                    icon.name: "download"

                    onTriggered: bar.packages.download_command_index()
                }
            ]
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: bar.packages.command_mode && bar.packages.command_state === Kinds.commandError
            type: Kirigami.MessageType.Warning
            text: Helpers.richText(bar.packages.command_error)
        }

        Kirigami.InlineMessage {
            id: stageMessage

            Layout.fillWidth: true
            visible: bar.stageError.length > 0
            type: Kirigami.MessageType.Error
            text: Helpers.richText(bar.stageError)
            showCloseButton: true

            onVisibleChanged: if (!stageMessage.visible)
                bar.stageErrorDismissed()
        }
    }
}
