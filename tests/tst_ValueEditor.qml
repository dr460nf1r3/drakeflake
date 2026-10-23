import QtQuick
import QtTest

import "../src/qml/components"

Item {
    id: root

    width: 800
    height: 600

    Component {
        id: valueEditorComponent

        ValueEditor {
            width: 400
            height: 300
        }
    }

    SignalSpy {
        id: editedSpy

        signalName: "edited"
    }

    TestCase {
        function test_componentExists(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
        }

        function test_kindDefaultsToRaw(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({});
            compare(editor.kind, "raw");
        }

        function test_kindReflectsInfo(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "bool"
                });
            compare(editor.kind, "bool");
        }

        function test_nullableDefaultsFalse(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "string"
                });
            compare(editor.nullable, false);
            compare(editor.isNull, false);
        }

        function test_isNullWhenNullableAndNullText(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "string",
                    nullable: true
                });
            editor.text = "null";
            compare(editor.nullable, true);
            compare(editor.isNull, true);
        }

        function test_isNullFalseForOtherText(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "string",
                    nullable: true
                });
            editor.text = "hello";
            compare(editor.isNull, false);
        }

        function test_spinnableIntRange(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "int",
                    min: 0,
                    max: 100
                });
            compare(editor.spinnable, true);
        }

        function test_spinnableFalseWithoutBounds(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "int"
                });
            compare(editor.spinnable, false);
        }

        function test_spinnableFalseOutside32Bit(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "int",
                    min: -3000000000,
                    max: 3000000000
                });
            compare(editor.spinnable, false);
        }

        function test_spinnableFalseForNonInt(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "float",
                    min: 0,
                    max: 100
                });
            compare(editor.spinnable, false);
        }

        function test_textWritesAndReads(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.text = "hello";
            compare(editor.text, "hello");
        }

        function test_nullCheckReportsNull(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "string",
                    nullable: true
                });
            editor.text = "hello";
            let nullCheck = findChild(editor, "nullCheck");
            verify(!!nullCheck, "Object exists");
            let field = findChild(editor, "valueField");
            verify(!!field, "Object exists");
            verify(waitForRendering(editor));
            editedSpy.target = editor;
            editedSpy.clear();
            mouseClick(nullCheck);
            tryCompare(editedSpy, "count", 1);
            compare(editedSpy.signalArguments[0][0], "null");
        }

        function test_boolSwitchReports(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "bool"
                });
            editor.text = "false";
            let boolSwitch = findChild(editor, "boolSwitch");
            verify(!!boolSwitch, "Object exists");
            editedSpy.target = editor;
            editedSpy.clear();
            mouseClick(boolSwitch);
            tryCompare(editedSpy, "count", 1);
            compare(editedSpy.signalArguments[0][0], "true");
        }

        function test_spinBoxReportsValue(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "int",
                    min: 0,
                    max: 100
                });
            editor.text = "5";
            let spin = findChild(editor, "intSpin");
            verify(!!spin, "Object exists");
            editedSpy.target = editor;
            editedSpy.clear();
            mouseClick(spin.up.indicator);
            tryCompare(editedSpy, "count", 1);
            compare(editedSpy.signalArguments[0][0], "6");
            tryCompare(spin, "value", 6);
        }

        function test_enumActivatedReports(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "enum",
                    choices: ["a", "b"]
                });
            editor.text = "a";
            let enumCombo = findChild(editor, "enumCombo");
            verify(!!enumCombo, "Object exists");
            editedSpy.target = editor;
            editedSpy.clear();
            enumCombo.currentIndex = 1;
            enumCombo.activated(1);
            tryCompare(editedSpy, "count", 1);
            compare(editedSpy.signalArguments[0][0], "b");
        }

        function test_textFieldMirrorsEditorText(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "string"
                });
            editor.text = "hello";
            let field = findChild(editor, "valueField");
            verify(!!field, "Object exists");
            compare(field.text, "hello");
        }

        function test_areaMirrorsEditorText(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "raw"
                });
            editor.text = '{ a = 1; }';
            let area = findChild(editor, "area");
            verify(!!area, "Object exists");
            compare(area.text, '{ a = 1; }');
        }

        function test_isNullNeedsNullable(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "string"
                });
            editor.text = "null";
            compare(editor.isNull, false);
        }

        function test_spinnableNeedsBothBounds(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "int",
                    min: 0
                });
            compare(editor.spinnable, false);
        }

        function test_spinnable32BitBounds(): void {
            let editor = createTemporaryObject(valueEditorComponent, root);
            verify(!!editor, "Component exists");
            editor.info = ({
                    kind: "int",
                    min: -2147483647,
                    max: 2147483647
                });
            compare(editor.spinnable, true);
            editor.info = ({
                    kind: "int",
                    min: -2147483648,
                    max: 2147483647
                });
            compare(editor.spinnable, false);
        }

        name: "ValueEditorTests"
        when: windowShown
    }
}
