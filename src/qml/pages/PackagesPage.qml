pragma ComponentBehavior: Bound

import QtQuick
import org.kde.kirigami as Kirigami

import "../components"
import "packages"

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var packages: page.app.packageModel
    readonly property var changes: page.app.changeModel
    property string stageError: ""
    readonly property var setList: {
        page.packages.revision;
        return page.packages.package_sets();
    }

    function stage(attr: string, add: bool): void {
        page.stageError = page.changes.stage_package(attr, add);
    }

    function focusSearch(): void {
        searchBar.focusSearch();
    }

    title: qsTr("Packages")

    header: PackageSearchBar {
        id: searchBar

        app: page.app
        packages: page.packages
        query: page.app.packageQuery
        onlyInstalled: page.app.packageOnlyInstalled
        setList: page.setList
        stageError: page.stageError

        onQueryEdited: text => page.app.packageQuery = text
        onOnlyInstalledToggled: checked => {
            page.app.packageOnlyInstalled = checked;
            page.packages.set_only_installed(checked);
        }
        onStageErrorDismissed: page.stageError = ""
    }

    ListView {
        id: list

        model: page.packages
        currentIndex: -1
        reuseItems: true

        delegate: PackageRow {
            id: row

            width: ListView.view.width
            app: page.app
            changes: page.changes
            packageSet: page.packages.package_set

            onStageRequested: add => page.stage(row.attr, add)
            onUndoRequested: page.changes.unstage("package:" + row.attr)
            onClicked: {
                list.currentIndex = row.index;
                page.app.showPackage(row.attr);
            }
        }
        footer: ShowMoreFooter {
            shown: list.count
            source: page.packages
        }

        PackagePlaceholder {
            anchors.centerIn: parent
            width: parent.width - Kirigami.Units.gridUnit * 4
            visible: list.count === 0
            packages: page.packages
            query: page.app.packageQuery
        }
    }
}
