import QtQuick
import QtTest

import "../src/qml/pages/overview"

Item {
    id: root

    width: 800
    height: 600

    Component {
        id: resultRowComponent

        ResultRow {
            width: 400
            height: 60
        }
    }

    SignalSpy {
        id: chosenSpy

        signalName: "chosen"
    }

    TestCase {
        function headerRow(more: var): var {
            let row = createTemporaryObject(resultRowComponent, root, {
                modelData: {
                    header: true,
                    group: {
                        title: "Pages",
                        more: more !== undefined ? more : "All 3",
                        moreOpen: {
                            kind: "section",
                            section: "pages"
                        }
                    }
                },
                index: 0,
                current: false
            });
            verify(!!row, "Component exists");
            return row;
        }

        function hitRow(): var {
            let row = createTemporaryObject(resultRowComponent, root, {
                modelData: {
                    header: false,
                    item: {
                        title: "Options",
                        subtitle: "sub",
                        trailing: "",
                        mono: true,
                        open: {
                            kind: "option",
                            name: "services.foo"
                        }
                    }
                },
                index: 1,
                current: false
            });
            verify(!!row, "Component exists");
            return row;
        }

        function test_componentExists(): void {
            hitRow();
        }

        function test_headerRoutesGroup(): void {
            let row = headerRow();
            compare(row.header, true);
            compare(row.group.title, "Pages");
            compare(row.item, null);
        }

        function test_hitRoutesItem(): void {
            let row = hitRow();
            compare(row.header, false);
            compare(row.item.title, "Options");
            compare(row.group, null);
        }

        function test_moreButtonChooses(): void {
            let row = headerRow();
            let more = findChild(row, "moreButton");
            verify(!!more, "Object exists");
            verify(waitForRendering(row));
            chosenSpy.target = row;
            chosenSpy.clear();
            mouseClick(more);
            tryCompare(chosenSpy, "count", 1);
            compare(chosenSpy.signalArguments[0][0].kind, "section");
            compare(chosenSpy.signalArguments[0][0].section, "pages");
        }

        function test_hitClickChooses(): void {
            let row = hitRow();
            let hit = findChild(row, "hit");
            verify(!!hit, "Object exists");
            verify(waitForRendering(row));
            chosenSpy.target = row;
            chosenSpy.clear();
            mouseClick(hit);
            tryCompare(chosenSpy, "count", 1);
            compare(chosenSpy.signalArguments[0][0].kind, "option");
            compare(chosenSpy.signalArguments[0][0].name, "services.foo");
        }

        function test_moreHiddenWithoutMore(): void {
            let row = headerRow("");
            let more = findChild(row, "moreButton");
            verify(!!more, "Object exists");
            compare(more.visible, false);
        }

        name: "ResultRowTests"
        when: windowShown
    }
}
