import QtQuick
import QtTest
import org.kde.kirigami as Kirigami

import "../src/qml/components"

Item {
    id: root

    width: 800
    height: 600

    Component {
        id: selectionBackgroundComponent

        SelectionBackground {
            width: 200
            height: 40
        }
    }

    TestCase {
        function test_componentExists(): void {
            let background = createTemporaryObject(selectionBackgroundComponent, root);
            verify(!!background, "Component exists");
        }

        function test_plainIsTransparent(): void {
            let background = createTemporaryObject(selectionBackgroundComponent, root);
            verify(!!background, "Component exists");
            compare(background.color, '#00000000');
        }

        function test_selectedTints(): void {
            let background = createTemporaryObject(selectionBackgroundComponent, root);
            verify(!!background, "Component exists");
            background.selected = true;
            compare(background.color, Qt.alpha(background.Kirigami.Theme.highlightColor, 0.16));
        }

        function test_hoveredTintsLess(): void {
            let background = createTemporaryObject(selectionBackgroundComponent, root);
            verify(!!background, "Component exists");
            background.hovered = true;
            compare(background.color, Qt.alpha(background.Kirigami.Theme.highlightColor, 0.07));
        }

        function test_selectedWinsOverHovered(): void {
            let background = createTemporaryObject(selectionBackgroundComponent, root);
            verify(!!background, "Component exists");
            background.hovered = true;
            background.selected = true;
            compare(background.color, Qt.alpha(background.Kirigami.Theme.highlightColor, 0.16));
        }

        function test_accentMarksSelected(): void {
            let background = createTemporaryObject(selectionBackgroundComponent, root);
            verify(!!background, "Component exists");
            let accent = findChild(background, "accent");
            verify(!!accent, "Object exists");
            background.selected = true;
            tryCompare(accent, "visible", true);
            background.selected = false;
            tryCompare(accent, "visible", false);
        }

        name: "SelectionBackgroundTests"
        when: windowShown
    }
}
