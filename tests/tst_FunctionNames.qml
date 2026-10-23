import QtQuick
import QtTest

import "../src/qml/pages/functions/names.js" as Names

Item {
    id: root

    width: 200
    height: 200

    TestCase {
        function test_kindLabels(): void {
            compare(Names.kindLabel("string"), qsTr("String"));
            compare(Names.kindLabel("number"), qsTr("Number"));
            compare(Names.kindLabel("bool"), qsTr("Boolean"));
            compare(Names.kindLabel("derivation"), qsTr("Derivation"));
        }

        function test_kindLabelAny(): void {
            compare(Names.kindLabel("any"), qsTr("Any type (a, b, …)"));
        }

        function test_kindLabelUnknown(): void {
            compare(Names.kindLabel("tofu"), qsTr("Anything"));
        }

        function test_kindOptionsAnythingFirst(): void {
            let options = Names.kindOptions(["string", "bool"]);
            compare(options.length, 3);
            compare(options[0].value, "");
            compare(options[0].text, qsTr("Anything"));
            compare(options[1].value, "string");
            compare(options[1].text, qsTr("String"));
            compare(options[2].value, "bool");
            compare(options[2].text, qsTr("Boolean"));
        }

        function test_namespaceNotes(): void {
            compare(Names.namespaceNote("builtins"), qsTr("Built into Nix"));
            compare(Names.namespaceNote("lib.strings"), qsTr("Text"));
            compare(Names.namespaceNote("pkgs"), qsTr("Builders and fetchers"));
        }

        function test_namespaceNoteUnknown(): void {
            compare(Names.namespaceNote("lib.tofu"), "");
        }

        function test_linkedFunctionLibrary(): void {
            compare(Names.linkedFunction("#function-library-lib.strings.concat"), "lib.strings.concat");
        }

        function test_linkedFunctionBuiltin(): void {
            compare(Names.linkedFunction("#builtins-map"), "builtins.map");
        }

        function test_linkedFunctionWebLink(): void {
            compare(Names.linkedFunction("https://example.com"), "");
        }

        function test_kindOptionsEmpty(): void {
            let options = Names.kindOptions([]);
            compare(options.length, 1);
            compare(options[0].value, "");
        }

        function test_linkedFunctionEmptyCapture(): void {
            compare(Names.linkedFunction("#function-library-"), "");
        }

        name: "FunctionNamesTests"
        when: windowShown
    }
}
