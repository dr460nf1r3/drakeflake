import QtQuick
import QtTest

import "../src/qml/pages/overview/entries.js" as Entries
import "../src/qml/pages/overview/results.js" as Search

Item {
    id: root

    width: 200
    height: 200

    TestCase {
        function test_wordsSplitAndLowercase(): void {
            compare(Search.words("  Hello   World ").join(","), "hello,world");
        }

        function test_wordsEmpty(): void {
            compare(Search.words("   ").length, 0);
        }

        function test_matchesEveryWord(): void {
            verify(Search.matches("foo bar", "Foo something BAR"));
            verify(!Search.matches("foo missing", "Foo something bar"));
        }

        function test_matchesEmptyQuery(): void {
            verify(Search.matches("", "anything here"));
        }

        function test_filterEntriesRanksTitleStartFirst(): void {
            let first = Entries.entry("Options", "sub", "kw", Entries.section("options"));
            let second = Entries.entry("My Options", "sub", "kw", Entries.section("options"));
            let found = Search.filterEntries([second, first], "opt");
            compare(found.length, 2);
            compare(found[0].title, qsTr("Options"));
        }

        function test_filterEntriesDropsNonMatches(): void {
            let hit = Entries.entry("Options", "sub", "kw", Entries.section("options"));
            let miss = Entries.entry("Packages", "add software", "install", Entries.section("packages"));
            let found = Search.filterEntries([hit, miss], "opt");
            compare(found.length, 1);
            compare(found[0].title, qsTr("Options"));
        }

        function test_groupTruncatesToThree(): void {
            let grouped = Search.group("k", "t", [1, 2, 3, 4, 5], 10, "All", null);
            compare(grouped.items.length, 3);
            compare(grouped.total, 10);
            compare(grouped.more, "All");
        }

        function test_groupNoMoreWhenComplete(): void {
            let grouped = Search.group("k", "t", [1, 2], 2, "All", null);
            compare(grouped.items.length, 2);
            compare(grouped.more, "");
        }

        function test_rowsSkipEmptyGroups(): void {
            let full = Search.group("a", "A", [
                {
                    title: "x"
                }
            ], 1, "", null);
            let empty = Search.group("b", "B", [], 0, "", null);
            let rows = Search.rows([empty, full]);
            compare(rows.length, 2);
            verify(rows[0].header);
            verify(!rows[1].header);
        }

        function test_pagesHaveSections(): void {
            let pages = Entries.pages();
            verify(pages.length > 0);
            let withSection = pages.filter(page => page.open && page.open.section && page.open.section.length > 0);
            compare(withSection.length, pages.length);
        }

        function test_actionsHaveKinds(): void {
            let actions = Entries.actions();
            verify(actions.length > 0);
            let withKind = actions.filter(action => action.open && action.open.kind && action.open.kind.length > 0);
            compare(withKind.length, actions.length);
        }

        function test_settingsOpenSettingsSection(): void {
            let settings = Entries.settings();
            verify(settings.length > 0);
            let inSettings = settings.filter(entry => entry.open && entry.open.section === "settings");
            compare(inSettings.length, settings.length);
        }

        function test_entryGroupMapsEntries(): void {
            let grouped = Search.entryGroup("pages", "T", Entries.pages().slice(0, 2), "");
            compare(grouped.items.length, 2);
            compare(grouped.total, 2);
            compare(grouped.items[0].trailing, "");
            compare(grouped.items[0].mono, false);
        }

        name: "OverviewSearchTests"
        when: windowShown
    }
}
