import QtQuick
import QtTest

import "../src/qml/main/helpers.js" as Helpers

Item {
    id: root

    width: 200
    height: 200

    TestCase {
        function test_docTextEmpty(): void {
            compare(Helpers.docText(""), "");
        }

        function test_docTextStripsImageToAltText(): void {
            compare(Helpers.docText("See ![logo](https://example.com/a.png) here"), "See logo here");
        }

        function test_docTextStripsReferenceImage(): void {
            compare(Helpers.docText("See ![logo][ref] here"), "See logo here");
        }

        function test_docTextRewritesMystRole(): void {
            compare(Helpers.docText("Enable {option}`services.foo` now"), "Enable `services.foo` now");
        }

        function test_docTextRewritesAdmonitionHeader(): void {
            compare(Helpers.docText("::: {.warning}"), "**warning:**");
        }

        function test_escapeHtml(): void {
            compare(Helpers.escapeHtml('<a href="x&y">'), '&lt;a href=&quot;x&amp;y&quot;&gt;');
        }

        function test_unescapeHtml(): void {
            compare(Helpers.unescapeHtml("&lt;b&gt; &amp; &quot;q&quot;"), '<b> & "q"');
        }

        function test_richTextWrapsAndEscapes(): void {
            compare(Helpers.richText("a<b"), "<span>a&lt;b</span>");
        }

        function test_linkEscapesUrlAndLabel(): void {
            compare(Helpers.link("https://example.com/?a=1&b=2", 'A "B"'), '<a href="https://example.com/?a=1&amp;b=2">A &quot;B&quot;</a>');
        }

        function test_shortRevTruncates(): void {
            compare(Helpers.shortRev("abcdef1234567890"), "abcdef123456");
        }

        function test_shortRevEmpty(): void {
            compare(Helpers.shortRev(""), "");
        }

        function test_matchingThemePicksLongest(): void {
            let themes = [
                {
                    name: "Catppuccin"
                },
                {
                    name: "Catppuccin Mocha"
                }
            ];
            compare(Helpers.matchingHighlightTheme("catppuccin-mocha-mauve", themes), "Catppuccin Mocha");
        }

        function test_matchingThemeNoMatch(): void {
            let themes = [
                {
                    name: "Catppuccin Mocha"
                }
            ];
            compare(Helpers.matchingHighlightTheme("breeze dark", themes), "");
        }

        function test_ageInDaysNow(): void {
            compare(Helpers.ageInDays(Date.now() / 1000), 0);
        }

        function test_docTextPassesPlainThrough(): void {
            compare(Helpers.docText("Just some text"), "Just some text");
        }

        function test_docTextUndefined(): void {
            compare(Helpers.docText(undefined), "");
        }

        name: "HelpersTests"
        when: windowShown
    }
}
