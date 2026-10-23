import QtQuick
import QtTest

import "../src/qml/pages/packages"

Item {
    id: root

    width: 800
    height: 600

    Component {
        id: packageRowComponent

        PackageRow {
            width: 600
            height: 140
        }
    }

    SignalSpy {
        id: stageSpy

        signalName: "stageRequested"
    }

    SignalSpy {
        id: undoSpy

        signalName: "undoRequested"
    }

    TestCase {
        function rowWith(attr: var, states: var, installed: var, packageSet: var, description: var): var {
            let changes = {
                revision: 0,
                package_state: name => states[name] || ""
            };
            let row = createTemporaryObject(packageRowComponent, root, {
                index: 0,
                attr: attr,
                version: "1.0",
                description: description !== undefined ? description : "description",
                installed: installed,
                app: {},
                changes: changes,
                packageSet: packageSet || ""
            });
            verify(!!row, "Component exists");
            return row;
        }

        function test_componentExists(): void {
            rowWith("firefox", {}, false);
        }

        function test_statePassesThrough(): void {
            let row = rowWith("firefox", {
                firefox: "add"
            }, false);
            compare(row.pkgState, "add");
        }

        function test_unstagedStateEmpty(): void {
            let row = rowWith("firefox", {}, false);
            compare(row.pkgState, "");
        }

        function test_attrShortensSetPrefix(): void {
            let row = rowWith("kde.kate", {}, false, "kde");
            let label = findChild(row, "attrLabel");
            verify(!!label, "Object exists");
            compare(label.text, "kate");
        }

        function test_attrKeepsFullWithoutSet(): void {
            let row = rowWith("firefox", {}, false, "");
            let label = findChild(row, "attrLabel");
            verify(!!label, "Object exists");
            compare(label.text, "firefox");
        }

        function test_addButtonWhenUnstaged(): void {
            let row = rowWith("firefox", {}, false);
            verify(waitForRendering(row));
            let add = findChild(row, "addButton");
            verify(!!add, "Object exists");
            compare(add.visible, true);
            let undo = findChild(row, "undoButton");
            verify(!!undo, "Object exists");
            compare(undo.visible, false);
        }

        function test_removeButtonWhenManaged(): void {
            let row = rowWith("firefox", {
                firefox: "managed"
            }, true);
            verify(waitForRendering(row));
            let remove = findChild(row, "removeButton");
            verify(!!remove, "Object exists");
            compare(remove.visible, true);
        }

        function test_noButtonsWhenInstalledUnstaged(): void {
            let row = rowWith("firefox", {}, true);
            verify(waitForRendering(row));
            let add = findChild(row, "addButton");
            verify(!!add, "Object exists");
            compare(add.visible, false);
            let undo = findChild(row, "undoButton");
            verify(!!undo, "Object exists");
            compare(undo.visible, false);
        }

        function test_addClickStages(): void {
            let row = rowWith("firefox", {}, false);
            let add = findChild(row, "addButton");
            verify(!!add, "Object exists");
            verify(waitForRendering(row));
            stageSpy.target = row;
            stageSpy.clear();
            mouseClick(add);
            tryCompare(stageSpy, "count", 1);
            compare(stageSpy.signalArguments[0][0], true);
        }

        function test_removeClickUnstages(): void {
            let row = rowWith("firefox", {
                firefox: "managed"
            }, true);
            let remove = findChild(row, "removeButton");
            verify(!!remove, "Object exists");
            verify(waitForRendering(row));
            stageSpy.target = row;
            stageSpy.clear();
            mouseClick(remove);
            tryCompare(stageSpy, "count", 1);
            compare(stageSpy.signalArguments[0][0], false);
        }

        function test_undoClickUndoes(): void {
            let row = rowWith("firefox", {
                firefox: "add"
            }, false);
            let undo = findChild(row, "undoButton");
            verify(!!undo, "Object exists");
            verify(waitForRendering(row));
            undoSpy.target = row;
            undoSpy.clear();
            mouseClick(undo);
            tryCompare(undoSpy, "count", 1);
        }

        function test_unknownStateHidesButtons(): void {
            let row = rowWith("firefox", {
                firefox: "weird"
            }, false);
            verify(waitForRendering(row));
            for (const name of ["addButton", "removeButton", "undoButton"]) {
                let button = findChild(row, name);
                verify(!!button, "Object exists");
                compare(button.visible, false);
            }
        }

        function test_descriptionHiddenWhenEmpty(): void {
            let row = rowWith("firefox", {}, false, "", "");
            let label = findChild(row, "descriptionLabel");
            verify(!!label, "Object exists");
            compare(label.visible, false);
        }

        name: "PackageRowTests"
        when: windowShown
    }
}
