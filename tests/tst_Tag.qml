import QtQuick
import QtTest
import org.kde.kirigami as Kirigami

import "../src/qml/components"

Item {
    id: root

    width: 800
    height: 600

    Component {
        id: tagComponent

        Tag {
            width: 160
            height: 32
        }
    }

    TestCase {
        function test_componentExists(): void {
            let tag = createTemporaryObject(tagComponent, root);
            verify(!!tag, "Component exists");
        }

        function test_textWritesAndReads(): void {
            let tag = createTemporaryObject(tagComponent, root);
            verify(!!tag, "Component exists");
            tag.text = qsTr("installed");
            compare(tag.text, qsTr("installed"));
        }

        function test_toneDefaultsNeutral(): void {
            let tag = createTemporaryObject(tagComponent, root);
            verify(!!tag, "Component exists");
            compare(tag.tone, "neutral");
        }

        function test_toneWritesAndReads(): void {
            let tag = createTemporaryObject(tagComponent, root);
            verify(!!tag, "Component exists");
            tag.tone = "positive";
            compare(tag.tone, "positive");
        }

        function test_toneColorRoutesRoles(): void {
            let tag = createTemporaryObject(tagComponent, root);
            verify(!!tag, "Component exists");
            tag.tone = "positive";
            compare(tag.toneColor, tag.Kirigami.Theme.positiveTextColor);
            tag.tone = "negative";
            compare(tag.toneColor, tag.Kirigami.Theme.negativeTextColor);
        }

        function test_unknownToneFallsBack(): void {
            let tag = createTemporaryObject(tagComponent, root);
            verify(!!tag, "Component exists");
            tag.tone = "tofu";
            compare(tag.toneColor, tag.Kirigami.Theme.disabledTextColor);
        }

        function test_hoverMarksHovered(): void {
            let tag = createTemporaryObject(tagComponent, root);
            verify(!!tag, "Component exists");
            let hover = findChild(tag, "hover");
            verify(!!hover, "Object exists");
            mouseMove(tag, tag.width / 2, tag.height / 2);
            tryCompare(hover, "hovered", true);
        }

        function test_emptyToneFallsBack(): void {
            let tag = createTemporaryObject(tagComponent, root);
            verify(!!tag, "Component exists");
            tag.tone = "";
            compare(tag.toneColor, tag.Kirigami.Theme.disabledTextColor);
        }

        name: "TagTests"
        when: windowShown
    }
}
