import QtQuick
import QtTest

import "../src/qml/components"

Item {
    id: root

    width: 200
    height: 200

    Component {
        id: themeColorsComponent

        ThemeColors {
        }
    }

    TestCase {
        function test_componentExists(): void {
            let colors = createTemporaryObject(themeColorsComponent, root);
            verify(!!colors, "Component exists");
        }

        function test_defaultTransparent(): void {
            let colors = createTemporaryObject(themeColorsComponent, root);
            verify(!!colors, "Component exists");
            compare(colors.background, '#00000000');
            compare(colors.text, '#00000000');
        }

        function test_defaultUnloaded(): void {
            let colors = createTemporaryObject(themeColorsComponent, root);
            verify(!!colors, "Component exists");
            verify(!colors.loaded);
            compare(colors.ansi.length, 0);
        }

        function test_emptyThemeNameStaysUnloaded(): void {
            let colors = createTemporaryObject(themeColorsComponent, root);
            verify(!!colors, "Component exists");
            colors.themeName = "";
            verify(!colors.loaded);
            compare(colors.background, '#00000000');
        }

        function test_unknownThemeNameStaysUnloaded(): void {
            let colors = createTemporaryObject(themeColorsComponent, root);
            verify(!!colors, "Component exists");
            colors.themeName = "No Such Theme";
            verify(!colors.loaded);
            compare(colors.background, '#00000000');
        }

        name: "ThemeColorsTests"
        when: windowShown
    }
}
