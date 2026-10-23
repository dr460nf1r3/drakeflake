import QtQuick
import QtTest
import org.kde.kirigami as Kirigami

import "../src/qml/components"

Item {
    id: root

    width: 800
    height: 600

    Component {
        id: codeBlockComponent

        CodeBlock {
            width: 400
            height: 200
        }
    }

    Component {
        id: plainBlockComponent

        CodeBlock {
            width: 400
            height: 200
            highlightNix: false
        }
    }

    TestCase {
        function test_componentExists(): void {
            let block = createTemporaryObject(codeBlockComponent, root);
            verify(!!block, "Component exists");
        }

        function test_textWritesAndReads(): void {
            let block = createTemporaryObject(codeBlockComponent, root);
            verify(!!block, "Component exists");
            block.text = "{ a = 1; }";
            compare(block.text, "{ a = 1; }");
        }

        function test_optionsWriteAndRead(): void {
            let block = createTemporaryObject(codeBlockComponent, root);
            verify(!!block, "Component exists");
            block.maximumLines = 5;
            block.themeName = "No Such Theme";
            compare(block.maximumLines, 5);
            compare(block.themeName, "No Such Theme");
        }

        function test_backgroundWithoutThemeColors(): void {
            let block = createTemporaryObject(codeBlockComponent, root);
            verify(!!block, "Component exists");
            let highlighter = findChild(block, "highlighter");
            verify(!!highlighter, "Object exists");
            verify(!highlighter.hasColors);
            compare(block.color, block.Kirigami.Theme.alternateBackgroundColor);
        }

        function test_textColorWithoutThemeColors(): void {
            let block = createTemporaryObject(codeBlockComponent, root);
            verify(!!block, "Component exists");
            let content = findChild(block, "content");
            verify(!!content, "Object exists");
            compare(content.color, block.Kirigami.Theme.textColor);
        }

        // Detaching at runtime crashes in the highlighter, so this covers
        // the creation-time path the pages use instead.
        function test_plainBlockHasNoHighlighter(): void {
            let block = createTemporaryObject(plainBlockComponent, root);
            verify(!!block, "Component exists");
            compare(block.highlightNix, false);
            let highlighter = findChild(block, "highlighter");
            verify(!!highlighter, "Object exists");
            compare(highlighter.textEdit, null);
        }

        function test_themeNameReachesHighlighter(): void {
            let block = createTemporaryObject(codeBlockComponent, root);
            verify(!!block, "Component exists");
            let highlighter = findChild(block, "highlighter");
            verify(!!highlighter, "Object exists");
            block.themeName = "No Such Theme";
            compare(highlighter.themeName, "No Such Theme");
        }

        name: "CodeBlockTests"
        when: windowShown
    }
}
