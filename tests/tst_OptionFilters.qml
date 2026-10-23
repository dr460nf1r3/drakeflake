import QtQuick
import QtTest

import "../src/qml/components"

Item {
    id: root

    width: 800
    height: 600

    Component {
        id: optionsStubComponent

        QtObject {
            property int size: 2
            property bool values_ready: true

            signal state_changed

            function namespaces(): var {
                return [
                    {
                        name: "a",
                        count: 1
                    },
                    {
                        name: "b",
                        count: 2
                    }
                ];
            }
        }
    }

    Component {
        id: optionFiltersComponent

        OptionFilters {
            width: 600
            height: 120
        }
    }

    SignalSpy {
        id: areaSpy

        signalName: "areaPicked"
    }

    SignalSpy {
        id: userSpy

        signalName: "onlyUserToggled"
    }

    SignalSpy {
        id: templateSpy

        signalName: "showTemplatedToggled"
    }

    TestCase {
        function filtersWithStub(): var {
            let stub = createTemporaryObject(optionsStubComponent, root);
            verify(!!stub, "Component exists");
            let filters = createTemporaryObject(optionFiltersComponent, root, {
                options: stub
            });
            verify(!!filters, "Component exists");
            return [stub, filters];
        }

        function test_componentExists(): void {
            let created = filtersWithStub();
            verify(created[1] !== null);
        }

        function test_areasLoadFromModel(): void {
            let created = filtersWithStub();
            let filters = created[1];
            compare(filters.loadedSize, 2);
            compare(filters.areas.length, 2);
        }

        function test_areaPickerListsAllAreas(): void {
            let created = filtersWithStub();
            let filters = created[1];
            let picker = findChild(filters, "areaPicker");
            verify(!!picker, "Object exists");
            compare(picker.options.length, 3);
            compare(picker.options[0].value, "");
            compare(picker.options[0].label, qsTr("All areas"));
            compare(picker.options[1].value, "a");
        }

        function test_areaPickerFollowsArea(): void {
            let created = filtersWithStub();
            let filters = created[1];
            let picker = findChild(filters, "areaPicker");
            verify(!!picker, "Object exists");
            filters.area = "b";
            compare(picker.currentValue, "b");
        }

        function test_pickReportsArea(): void {
            let created = filtersWithStub();
            let filters = created[1];
            let picker = findChild(filters, "areaPicker");
            verify(!!picker, "Object exists");
            verify(waitForRendering(filters));
            areaSpy.target = filters;
            userSpy.target = filters;
            templateSpy.target = filters;
            areaSpy.clear();
            userSpy.clear();
            templateSpy.clear();
            picker.choose("b");
            tryCompare(areaSpy, "count", 1);
            compare(areaSpy.signalArguments[0][0], "b");
            compare(userSpy.count, 0);
            compare(templateSpy.count, 0);
        }

        function test_userBoxReportsToggle(): void {
            let created = filtersWithStub();
            let filters = created[1];
            let userBox = findChild(filters, "userBox");
            verify(!!userBox, "Object exists");
            verify(waitForRendering(filters));
            areaSpy.target = filters;
            userSpy.target = filters;
            templateSpy.target = filters;
            areaSpy.clear();
            userSpy.clear();
            templateSpy.clear();
            mouseClick(userBox);
            tryCompare(userSpy, "count", 1);
            compare(userSpy.signalArguments[0][0], true);
            compare(areaSpy.count, 0);
            compare(templateSpy.count, 0);
        }

        function test_templateBoxReportsToggle(): void {
            let created = filtersWithStub();
            let filters = created[1];
            let templateBox = findChild(filters, "templateBox");
            verify(!!templateBox, "Object exists");
            verify(waitForRendering(filters));
            areaSpy.target = filters;
            userSpy.target = filters;
            templateSpy.target = filters;
            areaSpy.clear();
            userSpy.clear();
            templateSpy.clear();
            mouseClick(templateBox);
            tryCompare(templateSpy, "count", 1);
            compare(templateSpy.signalArguments[0][0], true);
            compare(areaSpy.count, 0);
            compare(userSpy.count, 0);
        }

        function test_reloadsWhenModelGrows(): void {
            let created = filtersWithStub();
            let stub = created[0];
            let filters = created[1];
            stub.size = 3;
            stub.state_changed();
            tryCompare(filters, "loadedSize", 3);
        }

        function test_userBoxDisabledUntilValuesReady(): void {
            let created = filtersWithStub();
            let stub = created[0];
            let filters = created[1];
            let userBox = findChild(filters, "userBox");
            verify(!!userBox, "Object exists");
            stub.values_ready = false;
            compare(userBox.enabled, false);
        }

        function test_pickReflectsLiveState(): void {
            let created = filtersWithStub();
            let filters = created[1];
            let picker = findChild(filters, "areaPicker");
            verify(!!picker, "Object exists");
            filters.onlyUser = true;
            filters.area = "a";
            areaSpy.target = filters;
            userSpy.target = filters;
            areaSpy.clear();
            userSpy.clear();
            picker.choose("b");
            tryCompare(areaSpy, "count", 1);
            compare(areaSpy.signalArguments[0][0], "b");
            compare(userSpy.count, 0);
        }

        name: "OptionFiltersTests"
        when: windowShown
    }
}
