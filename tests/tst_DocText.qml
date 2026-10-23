import QtQuick
import QtTest

import "../src/qml/main/helpers.js" as Helpers

Item {
    id: root

    width: 200
    height: 200

    Component {
        id: shownComponent

        TextEdit {
            textFormat: TextEdit.MarkdownText
            readOnly: true
        }
    }

    TestCase {
        function shown(markdown: string): string {
            const view = createTemporaryObject(shownComponent, root, {
                "text": Helpers.docText(markdown)
            });
            return view.getText(0, view.length).trim();
        }

        function test_rawImageIsEscaped(): void {
            const text = Helpers.docText("see <img src=\"http://evil/x.png\"> here");
            verify(text.indexOf("<img") < 0, text);
            compare(shown("see <img src=\"http://evil/x.png\"> here"), "see <img src=\"http://evil/x.png\"> here");
        }

        function test_rawAnchorIsEscaped(): void {
            const text = Helpers.docText("<a href=\"file:///etc/passwd\">open</a>");
            verify(text.indexOf("<a") < 0, text);
            compare(shown("<a href=\"file:///etc/passwd\">open</a>"), "<a href=\"file:///etc/passwd\">open</a>");
        }

        function test_rawBoldIsEscaped(): void {
            const text = Helpers.docText("a <b>bold</b> word");
            verify(text.indexOf("<b>") < 0, text);
            compare(shown("a <b>bold</b> word"), "a <b>bold</b> word");
        }

        function test_ampersandsAreEscaped(): void {
            compare(Helpers.docText("fish & chips &amp; more"), "fish &amp; chips &amp; more");
            compare(shown("fish & chips &amp; more &lt;b&gt;"), "fish & chips & more <b>");
        }

        function test_codeSpansKeepTheirTags(): void {
            compare(Helpers.docText("use `<b>` here, not <b>"), "use `<b>` here, not &lt;b>");
            compare(shown("use `<b>` here"), "use <b> here");
            compare(Helpers.docText("`a & b`"), "`a & b`");
        }

        function test_fencedCodeKeepsItsTags(): void {
            const fenced = "before <i>\n```html\n<img src=x> & <b>\n```\nafter <i>";
            compare(Helpers.docText(fenced), "before &lt;i>\n```html\n<img src=x> & <b>\n```\nafter &lt;i>");

            const tilde = "~~~\n<b>\n~~~\n<b>";
            compare(Helpers.docText(tilde), "~~~\n<b>\n~~~\n&lt;b>");
        }

        function test_autolinksStayLinks(): void {
            compare(Helpers.docText("see <https://example.org/a?x=1&y=2> now"), "see <https://example.org/a?x=1&y=2> now");
            compare(Helpers.docText("<http://a.b> and <b>"), "<http://a.b> and &lt;b>");
        }

        function test_markdownStillWorks(): void {
            compare(Helpers.docText("*em* **strong** [site](https://example.org)\n\n- item"), "*em* **strong** [site](https://example.org)\n\n- item");
            compare(shown("**strong**"), "strong");
        }

        function test_theGuideKeepsItsInlineHtml(): void {
            const guide = Helpers.guideText("Press <kbd>Ctrl</kbd> and <em>wait</em>.");
            compare(guide, "Press <kbd>Ctrl</kbd> and <em>wait</em>.");
        }

        function test_mystHandlingStays(): void {
            compare(Helpers.docText("{option}`services.foo` <x>"), "`services.foo` &lt;x>");
            compare(Helpers.docText("see ![alt](http://evil/x.png) <img src=y>"), "see alt &lt;img src=y>");
        }
    }
}
