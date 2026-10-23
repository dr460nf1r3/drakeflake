import QtQuick
import QtTest

import "../src/qml/components"

Item {
    id: root

    width: 200
    height: 200

    Component {
        id: settingTextComponent

        SettingText {
        }
    }

    TestCase {
        function test_componentExists(): void {
            let settingText = createTemporaryObject(settingTextComponent, root);
            verify(!!settingText, "Component exists");
        }

        function test_priorityLabels(): void {
            let settingText = createTemporaryObject(settingTextComponent, root);
            verify(!!settingText, "Component exists");
            compare(settingText.priorityLabel("override"), qsTr("Override"));
            compare(settingText.priorityLabel("default"), qsTr("Default"));
            compare(settingText.priorityLabel("normal"), qsTr("Normal"));
            compare(settingText.priorityLabel("force"), qsTr("Force"));
        }

        function test_priorityLabelsList(): void {
            let settingText = createTemporaryObject(settingTextComponent, root);
            verify(!!settingText, "Component exists");
            compare(settingText.priorityLabel("merge"), qsTr("Add (merge)"));
            compare(settingText.priorityLabel("before"), qsTr("Add first"));
            compare(settingText.priorityLabel("after"), qsTr("Add last"));
            compare(settingText.priorityLabel("replace"), qsTr("Replace"));
        }

        function test_priorityLabelUnknownPassesThrough(): void {
            let settingText = createTemporaryObject(settingTextComponent, root);
            verify(!!settingText, "Component exists");
            compare(settingText.priorityLabel("custom"), "custom");
        }

        function test_priorityHintKnown(): void {
            let settingText = createTemporaryObject(settingTextComponent, root);
            verify(!!settingText, "Component exists");
            compare(settingText.priorityHint("override"), qsTr("Wins over your configuration (lib.mkOverride 90)"));
            compare(settingText.priorityHint("force"), qsTr("Wins over everything (lib.mkForce)"));
        }

        function test_priorityHintUnknownEmpty(): void {
            let settingText = createTemporaryObject(settingTextComponent, root);
            verify(!!settingText, "Component exists");
            compare(settingText.priorityHint("custom"), "");
        }

        function test_summaryPriorityOnly(): void {
            let settingText = createTemporaryObject(settingTextComponent, root);
            verify(!!settingText, "Component exists");
            compare(settingText.summary("override", "", ""), qsTr("Override"));
        }

        function test_summaryExpressionOnly(): void {
            let settingText = createTemporaryObject(settingTextComponent, root);
            verify(!!settingText, "Component exists");
            compare(settingText.summary("", "expression", ""), qsTr("Nix expression"));
        }

        function test_summaryHostOnly(): void {
            let settingText = createTemporaryObject(settingTextComponent, root);
            verify(!!settingText, "Component exists");
            compare(settingText.summary("", "", "nixos"), qsTr("only on %1").arg("nixos"));
        }

        function test_summaryCombined(): void {
            let settingText = createTemporaryObject(settingTextComponent, root);
            verify(!!settingText, "Component exists");
            let expected = qsTr("Override") + " · " + qsTr("Nix expression") + " · " + qsTr("only on %1").arg("nixos");
            compare(settingText.summary("override", "expression", "nixos"), expected);
        }

        function test_summaryEmpty(): void {
            let settingText = createTemporaryObject(settingTextComponent, root);
            verify(!!settingText, "Component exists");
            compare(settingText.summary("", "", ""), "");
        }

        function test_summaryUnknownPriorityPassesThrough(): void {
            let settingText = createTemporaryObject(settingTextComponent, root);
            verify(!!settingText, "Component exists");
            compare(settingText.summary("custom", "", ""), "custom");
        }

        name: "SettingTextTests"
        when: windowShown
    }
}
