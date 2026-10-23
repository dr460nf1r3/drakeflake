import QtQuick
import QtTest

import "../src/qml/components"

Item {
    id: root

    width: 800
    height: 600

    Component {
        id: multiLineFieldComponent

        MultiLineField {
            width: 300
            height: 120
        }
    }

    SignalSpy {
        id: textEditedSpy

        signalName: "textEdited"
    }

    TestCase {
        function test_componentExists(): void {
            let field = createTemporaryObject(multiLineFieldComponent, root);
            verify(!!field, "Component exists");
        }

        function test_textWritesAndReads(): void {
            let field = createTemporaryObject(multiLineFieldComponent, root);
            verify(!!field, "Component exists");
            field.text = "hello";
            compare(field.text, "hello");
        }

        function test_placeholderWritesAndReads(): void {
            let field = createTemporaryObject(multiLineFieldComponent, root);
            verify(!!field, "Component exists");
            field.placeholderText = qsTr("One item per line");
            compare(field.placeholderText, qsTr("One item per line"));
        }

        function test_notEditingByDefault(): void {
            let field = createTemporaryObject(multiLineFieldComponent, root);
            verify(!!field, "Component exists");
            compare(field.editing, false);
        }

        function test_tapFocuses(): void {
            let field = createTemporaryObject(multiLineFieldComponent, root);
            verify(!!field, "Component exists");
            mouseClick(field);
            tryCompare(field, "editing", true);
        }

        function test_focusedAssignReports(): void {
            let field = createTemporaryObject(multiLineFieldComponent, root);
            verify(!!field, "Component exists");
            textEditedSpy.target = field;
            textEditedSpy.clear();
            mouseClick(field);
            tryCompare(field, "editing", true);
            field.text = "typed";
            tryCompare(textEditedSpy, "count", 1);
            compare(textEditedSpy.signalArguments[0][0], "typed");
        }

        function test_unfocusedAssignStaysSilent(): void {
            let field = createTemporaryObject(multiLineFieldComponent, root);
            verify(!!field, "Component exists");
            textEditedSpy.target = field;
            textEditedSpy.clear();
            field.text = "programmatic";
            compare(textEditedSpy.count, 0);
            compare(field.text, "programmatic");
        }

        function test_placeholderShowsWhenEmpty(): void {
            let field = createTemporaryObject(multiLineFieldComponent, root);
            verify(!!field, "Component exists");
            let placeholder = findChild(field, "placeholder");
            verify(!!placeholder, "Object exists");
            compare(placeholder.visible, true);
        }

        function test_placeholderHidesWithText(): void {
            let field = createTemporaryObject(multiLineFieldComponent, root);
            verify(!!field, "Component exists");
            let placeholder = findChild(field, "placeholder");
            verify(!!placeholder, "Object exists");
            field.text = "hello";
            compare(placeholder.visible, false);
        }

        name: "MultiLineFieldTests"
        when: windowShown
    }
}
