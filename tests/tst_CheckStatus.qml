import QtQuick
import QtTest

import "../src/qml/components"

Item {
    id: root

    width: 400
    height: 200

    Component {
        id: statusComponent

        CheckStatus {
        }
    }

    TestCase {
        function iconFor(check: var): var {
            let status = createTemporaryObject(statusComponent, root, {
                check: check,
                message: "message"
            });
            verify(status !== null);
            return status.children[0].source;
        }

        function test_iconPerCheck(): void {
            compare(iconFor("ok"), "dialog-ok");
            compare(iconFor("pending"), "view-refresh");
            compare(iconFor("unknown"), "help-hint");
            compare(iconFor("invalid"), "dialog-cancel");
            compare(iconFor("failed"), "dialog-cancel");
        }

        function test_messageIsShownAsPlainText(): void {
            let status = createTemporaryObject(statusComponent, root, {
                check: "failed",
                message: "<b>not bold</b>"
            });
            compare(status.children[1].text, "<b>not bold</b>");
        }

        name: "CheckStatusTests"
        when: windowShown
    }
}
