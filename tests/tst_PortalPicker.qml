import QtQuick
import QtTest

import "../src/qml/components"

Item {
    id: root

    width: 200
    height: 200

    Component {
        id: setupStubComponent

        QtObject {
            property var picked: null
            property var folderCalls: []
            property var fileCalls: []

            signal state_changed

            function pick_folder(purpose: var, title: var, folder: var): void {
                folderCalls.push([purpose, title, folder]);
            }

            function pick_save_file(purpose: var, title: var, folder: var, name: var): void {
                fileCalls.push([purpose, title, folder, name]);
            }
        }
    }

    Component {
        id: portalPickerComponent

        PortalPicker {
        }
    }

    SignalSpy {
        id: pickedSpy

        signalName: "picked"
    }

    TestCase {
        function pickerWithStub(folder: var): var {
            let stub = createTemporaryObject(setupStubComponent, root);
            verify(!!stub, "Component exists");
            let picker = createTemporaryObject(portalPickerComponent, root, {
                setupModel: stub,
                purpose: "import",
                title: "Pick",
                folder: folder,
                suggestedName: "drakeflake.nix",
                startFolder: "/tmp"
            });
            verify(!!picker, "Component exists");
            return [stub, picker];
        }

        function test_componentExists(): void {
            let created = pickerWithStub(true);
            verify(created[1] !== null);
        }

        function test_openFolderRoutes(): void {
            let created = pickerWithStub(true);
            let stub = created[0];
            let picker = created[1];
            picker.open();
            compare(stub.folderCalls.length, 1);
            compare(stub.folderCalls[0][0], "import");
            compare(stub.folderCalls[0][1], "Pick");
            compare(stub.folderCalls[0][2], "/tmp");
            compare(stub.fileCalls.length, 0);
        }

        function test_openFileRoutes(): void {
            let created = pickerWithStub(false);
            let stub = created[0];
            let picker = created[1];
            picker.open();
            compare(stub.fileCalls.length, 1);
            compare(stub.fileCalls[0][0], "import");
            compare(stub.fileCalls[0][3], "drakeflake.nix");
            compare(stub.folderCalls.length, 0);
        }

        function test_localPathDecodes(): void {
            let created = pickerWithStub(true);
            let picker = created[1];
            compare(picker.localPath("file:///a%20b/c.nix"), "/a b/c.nix");
        }

        function test_staleReplyIgnored(): void {
            let created = pickerWithStub(true);
            let stub = created[0];
            let picker = created[1];
            pickedSpy.target = picker;
            pickedSpy.clear();
            stub.picked = {
                serial: 0,
                purpose: "import",
                path: "/stale"
            };
            stub.state_changed();
            compare(pickedSpy.count, 0);
        }

        function test_foreignReplyIgnored(): void {
            let created = pickerWithStub(true);
            let stub = created[0];
            let picker = created[1];
            pickedSpy.target = picker;
            pickedSpy.clear();
            stub.picked = {
                serial: 1,
                purpose: "other",
                path: "/foreign"
            };
            stub.state_changed();
            compare(pickedSpy.count, 0);
        }

        function test_freshReplyPickedOnce(): void {
            let created = pickerWithStub(true);
            let stub = created[0];
            let picker = created[1];
            pickedSpy.target = picker;
            pickedSpy.clear();
            stub.picked = {
                serial: 2,
                purpose: "import",
                path: "/flake"
            };
            stub.state_changed();
            tryCompare(pickedSpy, "count", 1);
            compare(pickedSpy.signalArguments[0][0], "/flake");
            stub.state_changed();
            compare(pickedSpy.count, 1);
        }

        function test_nullReplyIgnored(): void {
            let created = pickerWithStub(true);
            let stub = created[0];
            let picker = created[1];
            pickedSpy.target = picker;
            pickedSpy.clear();
            stub.picked = null;
            stub.state_changed();
            compare(pickedSpy.count, 0);
        }

        function test_pathlessReplyConsumesSerial(): void {
            let created = pickerWithStub(true);
            let stub = created[0];
            let picker = created[1];
            pickedSpy.target = picker;
            pickedSpy.clear();
            stub.picked = {
                serial: 3,
                purpose: "import"
            };
            stub.state_changed();
            compare(pickedSpy.count, 0);
            stub.picked = {
                serial: 3,
                purpose: "import",
                path: "/late"
            };
            stub.state_changed();
            compare(pickedSpy.count, 0);
        }

        name: "PortalPickerTests"
        when: windowShown
    }
}
