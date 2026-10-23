pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers
import "results.js" as Results

ColumnLayout {
    id: search

    required property var app
    property string query: ""
    property var groups: []
    property var rows: []
    property int current: -1
    property string message: ""
    // The function index may arrive after the query was typed.
    readonly property bool functionsReady: search.app.functionModel.state === "ready"

    function refresh(): void {
        search.query = field.text.trim();
        search.message = "";
        search.app.packageModel.find_overview_command(search.query);
        search.app.searchModel.search_flake(search.query);
        search.update();
    }

    function searchFor(text: string): void {
        field.text = text;
        search.refresh();
    }

    function update(): void {
        if (search.query.length === 0) {
            search.groups = [];
            search.rows = [];
            search.current = -1;
            dropdown.close();
            return;
        }
        search.groups = Results.groups(search.app, search.query);
        search.rows = Results.rows(search.groups);
        search.current = search.rows.findIndex(row => !row.header);

        if (search.rows.length > 0)
            dropdown.open();
        else
            dropdown.close();
    }

    function move(step: int): void {
        let next = search.current;

        do
            next += step;
        while (next >= 0 && next < search.rows.length && search.rows[next].header)

        if (next >= 0 && next < search.rows.length) {
            search.current = next;
            results.positionViewAtIndex(next, ListView.Contain);
        }
    }

    function open(target: var): void {
        if (!target)
            return;

        const app = search.app;
        dropdown.close();

        switch (target.kind) {
        case "section":
            app.showSection(target.section);
            break;
        case "option":
            app.showOption(target.name);
            break;
        case "configured":
            app.showSection("configured");
            app.showOption(target.name, app.configuredModel);
            break;
        case "home":
            app.showSection("home");
            app.showOption(target.name, app.homeModel);
            break;
        case "package":
            app.showPackage(target.attr);
            break;
        case "function":
            app.showFunction(target.path);
            break;
        case "source":
            app.searchModel.open_hit(target.file, target.line);
            break;
        case "search":
            search.showSearch(target.section, target.query);
            break;
        case "rebuild":
            search.started(app.systemModel.rebuild(target.mode));
            break;
        case "update-inputs":
            search.started(app.systemModel.update_lock([]));
            break;
        case "setup":
            app.setupOpen = true;
            break;
        }
    }

    function showSearch(section: string, query: string): void {
        const app = search.app;

        if (section === "options") {
            app.optionQuery = query;
            app.optionModel.search(query);
        } else if (section === "configured") {
            app.configuredQuery = query;
            app.configuredModel.search(query);
        } else if (section === "home") {
            app.homeQuery = query;
            app.homeModel.search(query);
        } else if (section === "packages") {
            app.packageQuery = query;
            app.packageModel.search(query);
        } else if (section === "functions") {
            app.functionQuery = query;
            app.functionModel.reset();
            app.functionModel.search(query);
        }
        app.showSection(section);
    }

    function started(error: string): void {
        search.message = error;

        if (error.length === 0)
            search.app.showSection("log");
    }

    function focusSearch(): void {
        field.forceActiveFocus();
    }

    spacing: Kirigami.Units.smallSpacing

    onFunctionsReadyChanged: {
        if (search.functionsReady && search.query.length > 0)
            search.update();
    }

    Kirigami.SearchField {
        id: field

        focusSequences: []
        Layout.fillWidth: true
        placeholderText: qsTr("Search options, packages, functions, Home Manager, inputs…")
        autoAccept: false
        Accessible.name: qsTr("Search everything")

        onTextEdited: debounce.restart()
        onAccepted: {
            debounce.stop();

            if (dropdown.opened && search.current >= 0 && field.text.trim() === search.query)
                search.open(search.rows[search.current].item.open);
            else
                search.refresh();
        }
        Keys.onDownPressed: search.move(1)
        Keys.onUpPressed: search.move(-1)
        Keys.onEscapePressed: event => {
            if (dropdown.opened) {
                dropdown.close();
                event.accepted = true;
            } else {
                event.accepted = false;
            }
        }
        Component.onCompleted: field.forceActiveFocus()

        FieldMenu {
            target: field
        }

        Timer {
            id: debounce

            interval: 200

            onTriggered: search.refresh()
        }

        Controls.Popup {
            id: dropdown

            y: field.height + Kirigami.Units.smallSpacing
            width: field.width
            padding: 1
            // The field keeps the focus: typing goes on while it is open.
            focus: false
            closePolicy: Controls.Popup.CloseOnPressOutsideParent
            Kirigami.Theme.colorSet: Kirigami.Theme.View
            Kirigami.Theme.inherit: false

            background: Kirigami.ShadowedRectangle {
                color: Kirigami.Theme.backgroundColor
                radius: Kirigami.Units.cornerRadius
                border.width: 1
                border.color: Kirigami.ColorUtils.linearInterpolation(Kirigami.Theme.backgroundColor, Kirigami.Theme.textColor, 0.25)
                shadow.size: Kirigami.Units.largeSpacing * 2
                shadow.yOffset: Kirigami.Units.smallSpacing / 2
                shadow.color: Qt.rgba(0, 0, 0, 0.18)
            }
            contentItem: ListView {
                id: results

                implicitHeight: Math.min(results.contentHeight, Kirigami.Units.gridUnit * 24)
                clip: true
                model: search.rows
                boundsBehavior: Flickable.StopAtBounds

                delegate: ResultRow {
                    id: resultRow

                    current: resultRow.index === search.current
                    scrollInset: scrollBar.size < 1 ? scrollBar.width : 0

                    onChosen: target => search.open(target)
                }
                Controls.ScrollBar.vertical: Controls.ScrollBar {
                    id: scrollBar
                }

                // Smooth wheel scrolling like the pages (a plain ListView
                // jumps a whole step per wheel notch).
                Kirigami.WheelHandler {
                    target: results
                    filterMouseEvents: true
                }
            }
        }
    }

    Controls.Label {
        Layout.fillWidth: true
        visible: search.query.length > 0 && search.rows.length === 0
        text: qsTr("Nothing found for “%1”.").arg(search.query)
        textFormat: Text.PlainText
        horizontalAlignment: Text.AlignHCenter
        opacity: 0.75
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: search.message.length > 0
        type: Kirigami.MessageType.Error
        text: Helpers.richText(search.message)
    }

    Connections {
        function onState_changed(): void {
            const commands = search.app.packageModel.overview_commands;

            if (search.query.length > 0 && commands.command === search.query && commands.attrs.length > 0 && !search.groups.some(found => found.key === "commands"))
                search.update();
        }

        target: search.app.packageModel
    }

    Connections {
        function onState_changed(): void {
            if (search.query.length > 0 && search.app.searchModel.flake_hits.query === search.query)
                search.update();
        }

        target: search.app.searchModel
    }
}
