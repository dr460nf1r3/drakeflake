import QtQuick
import QtTest
import QtQuick.Controls

import "../src/qml/components"

Item {
    id: root

    width: 800
    height: 600

    Component {
        id: iconButtonComponent

        IconButton {
            width: 40
            height: 40
        }
    }

    TestCase {
        function test_componentExists(): void {
            let button = createTemporaryObject(iconButtonComponent, root);
            verify(!!button, "Component exists");
        }

        function test_textFeedsAssistiveTools(): void {
            let button = createTemporaryObject(iconButtonComponent, root);
            verify(!!button, "Component exists");
            button.text = qsTr("Refresh");
            compare(button.text, qsTr("Refresh"));
            compare(button.Accessible.name, qsTr("Refresh"));
            compare(button.ToolTip.text, qsTr("Refresh"));
        }

        function test_noBackground(): void {
            let button = createTemporaryObject(iconButtonComponent, root);
            verify(!!button, "Component exists");
            compare(button.background, null);
        }

        function test_iconNameRoutesToIcon(): void {
            let button = createTemporaryObject(iconButtonComponent, root);
            verify(!!button, "Component exists");
            button.icon.name = "list-add";
            compare(button.contentItem.source, "list-add");
        }

        function test_disabledDimsIcon(): void {
            let button = createTemporaryObject(iconButtonComponent, root);
            verify(!!button, "Component exists");
            button.enabled = false;
            compare(button.contentItem.opacity, 0.4);
        }

        function test_hoverHighlightsIcon(): void {
            let button = createTemporaryObject(iconButtonComponent, root);
            verify(!!button, "Component exists");
            button.hoverEnabled = true;
            mouseMove(button, button.width / 2, button.height / 2);
            tryCompare(button.contentItem, "opacity", 1.0);
        }

        name: "IconButtonTests"
        when: windowShown
    }
}
