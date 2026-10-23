import QtQuick
import QtTest

import "../src/qml/components"

Item {
    id: root

    width: 800
    height: 600

    Component {
        id: filterPickerComponent

        FilterPicker {
            width: 200
            height: 40
        }
    }

    SignalSpy {
        id: pickedSpy

        signalName: "picked"
    }

    TestCase {
        function test_componentExists(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
        }

        function test_currentLabelEmptyByDefault(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            compare(picker.currentLabel, "");
        }

        function test_currentLabelResolves(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            picker.options = [
                {
                    value: "a",
                    label: "Alpha"
                },
                {
                    value: "b",
                    label: "Beta"
                }
            ];
            picker.currentValue = "b";
            compare(picker.currentLabel, "Beta");
        }

        function test_currentLabelUnknownValue(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            picker.options = [
                {
                    value: "a",
                    label: "Alpha"
                }
            ];
            picker.currentValue = "missing";
            compare(picker.currentLabel, "");
        }

        function test_filterTextWritesAndReads(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            picker.filterText = "alp";
            compare(picker.filterText, "alp");
        }

        function test_popupClosedByDefault(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            compare(picker.popupOpen, false);
        }

        function test_chooseEmitsPicked(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            picker.options = [
                {
                    value: "a",
                    label: "Alpha"
                },
                {
                    value: "b",
                    label: "Beta"
                }
            ];
            picker.currentValue = "a";
            pickedSpy.target = picker;
            pickedSpy.clear();
            picker.choose("b");
            tryCompare(pickedSpy, "count", 1);
            compare(pickedSpy.signalArguments[0][0], "b");
        }

        function test_chooseSameValueStaysSilent(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            picker.options = [
                {
                    value: "a",
                    label: "Alpha"
                }
            ];
            picker.currentValue = "a";
            pickedSpy.target = picker;
            pickedSpy.clear();
            picker.choose("a");
            compare(pickedSpy.count, 0);
        }

        function test_openAndClosePopup(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            picker.open();
            tryCompare(picker, "popupOpen", true);
            picker.close();
            tryCompare(picker, "popupOpen", false);
        }

        function test_filterNarrowsList(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            picker.options = [
                {
                    value: "a",
                    label: "Alpha"
                },
                {
                    value: "b",
                    label: "Beta"
                },
                {
                    value: "c",
                    label: "Gamma"
                }
            ];
            picker.open();
            tryCompare(picker, "popupOpen", true);
            let list = findChild(picker, "list");
            verify(!!list, "Object exists");
            picker.filterText = "alp";
            tryCompare(list, "count", 1);
            picker.close();
            tryCompare(picker, "popupOpen", false);
        }

        function test_acceptedPicksFirstMatch(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            picker.options = [
                {
                    value: "a",
                    label: "Alpha"
                },
                {
                    value: "b",
                    label: "Beta"
                }
            ];
            picker.open();
            tryCompare(picker, "popupOpen", true);
            let filter = findChild(picker, "filter");
            verify(!!filter, "Object exists");
            pickedSpy.target = picker;
            pickedSpy.clear();
            picker.filterText = "bet";
            filter.accepted();
            tryCompare(pickedSpy, "count", 1);
            compare(pickedSpy.signalArguments[0][0], "b");
        }

        function test_delegateClickChooses(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            picker.options = [
                {
                    value: "a",
                    label: "Alpha"
                },
                {
                    value: "b",
                    label: "Beta"
                }
            ];
            picker.open();
            tryCompare(picker, "popupOpen", true);
            let list = findChild(picker, "list");
            verify(!!list, "Object exists");
            let choice = findChild(list, "choice");
            verify(!!choice, "Object exists");
            pickedSpy.target = picker;
            pickedSpy.clear();
            mouseClick(choice);
            tryCompare(pickedSpy, "count", 1);
            compare(pickedSpy.signalArguments[0][0], "a");
        }

        function test_openClearsFilter(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            picker.filterText = "alp";
            picker.open();
            tryCompare(picker, "popupOpen", true);
            tryCompare(picker, "filterText", "");
            picker.close();
            tryCompare(picker, "popupOpen", false);
        }

        function test_emptyFilterShowsHint(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            picker.options = [
                {
                    value: "a",
                    label: "Alpha"
                },
                {
                    value: "b",
                    label: "Beta"
                }
            ];
            picker.open();
            tryCompare(picker, "popupOpen", true);
            let list = findChild(picker, "list");
            verify(!!list, "Object exists");
            let hint = findChild(picker, "emptyHint");
            verify(!!hint, "Object exists");
            picker.filterText = "zzz";
            tryCompare(list, "count", 0);
            tryCompare(hint, "visible", true);
            picker.close();
            tryCompare(picker, "popupOpen", false);
        }

        function test_hintHiddenWithResults(): void {
            let picker = createTemporaryObject(filterPickerComponent, root);
            verify(!!picker, "Component exists");
            picker.options = [
                {
                    value: "a",
                    label: "Alpha"
                }
            ];
            picker.open();
            tryCompare(picker, "popupOpen", true);
            let hint = findChild(picker, "emptyHint");
            verify(!!hint, "Object exists");
            tryCompare(hint, "visible", false);
            picker.close();
            tryCompare(picker, "popupOpen", false);
        }

        name: "FilterPickerTests"
        when: windowShown
    }
}
