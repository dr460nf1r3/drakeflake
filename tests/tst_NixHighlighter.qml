import QtQuick
import QtTest
import org.kde.syntaxhighlighting

import "../src/qml/components"

Item {
    id: root

    width: 200
    height: 200

    Component {
        id: nixHighlighterComponent

        NixHighlighter {
        }
    }

    TestCase {
        function test_componentExists(): void {
            let highlighter = createTemporaryObject(nixHighlighterComponent, root);
            verify(!!highlighter, "Component exists");
        }

        function test_nixDefinition(): void {
            let highlighter = createTemporaryObject(nixHighlighterComponent, root);
            verify(!!highlighter, "Component exists");
            compare(highlighter.definition.name, "Nix");
        }

        function test_noColorsByDefault(): void {
            let highlighter = createTemporaryObject(nixHighlighterComponent, root);
            verify(!!highlighter, "Component exists");
            verify(!highlighter.hasColors);
            compare(highlighter.backgroundColor, '#00000000');
            compare(highlighter.textColor, '#00000000');
        }

        function test_optionsWriteAndRead(): void {
            let highlighter = createTemporaryObject(nixHighlighterComponent, root);
            verify(!!highlighter, "Component exists");
            highlighter.themeName = "No Such Theme";
            highlighter.dark = true;
            compare(highlighter.themeName, "No Such Theme");
            compare(highlighter.dark, true);
        }

        function test_unknownThemeKeepsFallback(): void {
            let highlighter = createTemporaryObject(nixHighlighterComponent, root);
            verify(!!highlighter, "Component exists");
            highlighter.themeName = "No Such Theme";
            verify(!highlighter.hasColors);
        }

        function test_unknownThemeFallsBackToDefault(): void {
            let highlighter = createTemporaryObject(nixHighlighterComponent, root);
            verify(!!highlighter, "Component exists");
            highlighter.themeName = "No Such Theme";
            compare(highlighter.theme, Repository.defaultTheme(Repository.LightTheme));
        }

        name: "NixHighlighterTests"
        when: windowShown
    }
}
