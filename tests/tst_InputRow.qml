import QtQuick
import QtTest

import "../src/qml/pages/inputs"

Item {
    id: root

    width: 800
    height: 600

    Component {
        id: inputRowComponent

        InputRow {
            width: 600
            height: 200
        }
    }

    SignalSpy {
        id: toggledSpy

        signalName: "toggled"
    }

    SignalSpy {
        id: updateSpy

        signalName: "updateRequested"
    }

    TestCase {
        function baseData(): var {
            return {
                name: "nixpkgs",
                kind: "git",
                follows: "",
                pinned: false,
                rev: "abcdef1234567890",
                source: "github:NixOS/nixpkgs",
                web_url: "https://github.com/NixOS/nixpkgs",
                commit_url: "https://github.com/NixOS/nixpkgs/commit/abcdef1234567890",
                last_modified: 1577836800,
                check: {
                    state: "available",
                    rev: "fedcba0987654321",
                    compare_url: ""
                },
                nested: []
            };
        }

        function rowWith(data: var, checking: var, busy: var, open: var): var {
            let app = {
                opened: [],
                openExternal: link => app.opened.push(link)
            };
            let row = createTemporaryObject(inputRowComponent, root, {
                modelData: data,
                app: app,
                checking: checking !== undefined ? checking : false,
                busy: busy !== undefined ? busy : false,
                open: open !== undefined ? open : false
            });
            verify(!!row, "Component exists");
            return row;
        }

        function test_componentExists(): void {
            rowWith(baseData());
        }

        function test_followsState(): void {
            let data = baseData();
            data.follows = "master";
            let row = rowWith(data);
            compare(row.inputState, "follows");
        }

        function test_checkingState(): void {
            let data = baseData();
            data.check = null;
            let row = rowWith(data, true);
            compare(row.inputState, "checking");
        }

        function test_unknownState(): void {
            let data = baseData();
            data.check = null;
            let row = rowWith(data, false);
            compare(row.inputState, "unknown");
        }

        function test_checkStatePassesThrough(): void {
            let data = baseData();
            data.check = {
                state: "failed",
                error: "boom"
            };
            let row = rowWith(data);
            compare(row.inputState, "failed");
        }

        function test_missingCheckAndNestedDefault(): void {
            let data = baseData();
            delete data.check;
            delete data.nested;
            let row = rowWith(data, false);
            compare(row.check, null);
            compare(row.nested.length, 0);
            compare(row.inputState, "unknown");
        }

        function test_backgroundTapToggles(): void {
            let data = baseData();
            data.nested = [
                {
                    name: "sub"
                }
            ];
            let row = rowWith(data);
            verify(waitForRendering(row));
            toggledSpy.target = row;
            toggledSpy.clear();
            mouseClick(row.background, 10, 10);
            tryCompare(toggledSpy, "count", 1);
        }

        function test_updateButtonRequests(): void {
            let row = rowWith(baseData());
            let button = findChild(row, "updateButton");
            verify(!!button, "Object exists");
            verify(waitForRendering(row));
            updateSpy.target = row;
            updateSpy.clear();
            mouseClick(button);
            tryCompare(updateSpy, "count", 1);
        }

        function test_busyDisablesUpdate(): void {
            let row = rowWith(baseData(), false, true);
            let button = findChild(row, "updateButton");
            verify(!!button, "Object exists");
            compare(button.enabled, false);
        }

        function test_pinnedHidesUpdate(): void {
            let data = baseData();
            data.pinned = true;
            let row = rowWith(data);
            let button = findChild(row, "updateButton");
            verify(!!button, "Object exists");
            compare(button.visible, false);
        }

        function test_followsHidesUpdate(): void {
            let data = baseData();
            data.follows = "master";
            let row = rowWith(data);
            let button = findChild(row, "updateButton");
            verify(!!button, "Object exists");
            compare(button.visible, false);
        }

        function test_tapWithoutNestedStaysSilent(): void {
            let row = rowWith(baseData());
            verify(waitForRendering(row));
            toggledSpy.target = row;
            toggledSpy.clear();
            mouseClick(row.background, 10, 10);
            compare(toggledSpy.count, 0);
        }

        name: "InputRowTests"
        when: windowShown
    }
}
