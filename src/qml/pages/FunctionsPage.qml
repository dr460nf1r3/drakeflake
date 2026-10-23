pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

import "functions"

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var functions: page.app.functionModel

    function focusSearch(): void {
        searchBar.focusSearch();
    }

    title: qsTr("Functions")

    header: FunctionSearchBar {
        id: searchBar

        clear: page.app.vibrancy
        functions: page.functions
        query: page.app.functionQuery

        onQueryEdited: text => page.app.functionQuery = text
    }

    Component.onCompleted: page.functions.load()

    ListView {
        id: list

        objectName: "functionList"
        model: page.functions
        currentIndex: -1
        reuseItems: true

        header: Item {
            width: ListView.view ? ListView.view.width : 0
            height: overview.visible ? overview.implicitHeight + Kirigami.Units.largeSpacing * 2 : 0

            FunctionsOverview {
                id: overview

                visible: page.functions.state === "ready" && !page.functions.browsing
                app: page.app

                onNamespaceChosen: name => page.functions.set_namespace(name)

                anchors {
                    left: parent.left
                    right: parent.right
                    top: parent.top
                    margins: Kirigami.Units.largeSpacing
                }
            }
        }
        delegate: FunctionRow {
            id: row

            width: ListView.view.width

            onClicked: {
                list.currentIndex = row.index;
                page.app.showFunction(row.path);
            }
        }
        footer: Item {
            width: ListView.view ? ListView.view.width : 0
            height: page.functions.has_more ? more.implicitHeight + Kirigami.Units.largeSpacing * 2 : 0

            Controls.Button {
                id: more

                anchors.centerIn: parent
                visible: page.functions.has_more
                text: qsTr("Show more (%1 left)").arg(page.functions.total - list.count)
                icon.name: "go-down"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels

                onClicked: page.functions.load_more()
            }
        }

        FunctionPlaceholder {
            anchors.centerIn: parent
            width: parent.width - Kirigami.Units.gridUnit * 4
            visible: list.count === 0 && (page.functions.state !== "ready" || page.functions.browsing)
            functions: page.functions
        }
    }
}
