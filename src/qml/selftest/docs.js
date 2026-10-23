.pragma library
.import QtQuick as QtQuick
.import "../main/helpers.js" as Helpers
.import "../pages/overview/results.js" as Results

function top(test) {
    return test.app.pageStack.items[test.app.pageStack.depth - 1];
}

function guideShown(test, file) {
    const page = top(test);
    return page && page.isGuide && page.file === file && page.guide.file === file && !page.settling;
}

function blocksOf(test, kind) {
    const found = [];
    const walk = item => {
        if (!item)
            return;
        if (item.objectName === "guide-" + kind)
            found.push(item);
        for (const child of item.children || [])
            walk(child);
    };
    walk(top(test));
    return found;
}

function helpActions(page) {
    const found = [];
    for (let i = 0; i < page.actions.length; i++) {
        if (page.actions[i].objectName === "helpAction")
            found.push(page.actions[i]);
    }
    return found;
}

function helpOf(page) {
    return helpActions(page)[0] || null;
}

function contentsCount(test) {
    return test.app.sectionPages.docs.flickable.count;
}

function searchDocs(test, text) {
    const field = test.findChild(test.app.sectionPages.docs.header, "docsSearch");
    test.check(field !== null, "the Documentation page has no search field");
    field.text = text;
    field.accepted();
}

function steps(test) {
    return [
        {
            name: "documentation section",
            run: () => test.app.globalDrawer.leaf("docs").trigger(),
            until: () => guideShown(test, "introduction.md"),
            delay: 800,
            then: () => {
                test.check(test.app.section === "docs", "the drawer entry did not open the Documentation section");
                const contents = test.app.sectionPages.docs.contents;
                test.check(contents.error === "", "SUMMARY.md not read: " + contents.error);
                test.check(contents.entries.some(entry => entry.file === "options.md" && entry.part === "Using the app"), "the contents lack Options under Using the app");
                test.check(test.app.sectionPages.docs.currentFile === "introduction.md", "the contents do not mark the open page");
                test.check(top(test).title === "Introduction", "the guide page is titled " + top(test).title);
                const images = blocksOf(test, "image");
                test.check(images.length >= 2, "the introduction shows " + images.length + " images");
                test.check(images.every(image => image.item.ready), "an image of the introduction did not load");
                test.check(images.every(image => image.item.width <= top(test).width), "an image is wider than its column");
                test.check(blocksOf(test, "heading").length > 3, "the introduction has no headings");
            }
        },
        {
            name: "screenshot documentation",
            run: () => test.shot("10f-docs-introduction"),
            delay: 800
        },
        {
            name: "documentation search",
            run: () => searchDocs(test, "impermanence"),
            until: () => contentsCount(test) === 1,
            then: () => {
                const docs = test.app.sectionPages.docs;
                test.check(docs.query === "impermanence", "the search field did not set the query: " + docs.query);
                test.check(docs.filtered[0].file === "impermanence.md", "the search found " + docs.filtered[0].file);
            }
        },
        {
            name: "documentation search cleared",
            run: () => searchDocs(test, ""),
            until: () => contentsCount(test) === test.app.sectionPages.docs.contents.entries.length
        },
        {
            name: "documentation code, notes and theme",
            run: () => test.app.showDoc("settings.md", ""),
            until: () => guideShown(test, "settings.md"),
            delay: 800,
            then: () => {
                const code = blocksOf(test, "code");
                test.check(code.length > 0, "settings.md shows no code block");
                test.check(code[0].item.themeName === test.app.highlightTheme, "the code block ignores the highlighting theme");
                const texts = blocksOf(test, "text");
                test.check(texts.length > 0, "settings.md shows no text block");
                const text = texts[0].item;
                test.check(text.textFormat === QtQuick.Text.MarkdownText, "guide text is not rendered as Markdown");
                test.check(Qt.colorEqual(text.color, test.themeColor("textColor")), "guide text has " + text.color + ", not the theme's text colour");
                test.check(top(test).flickable.contentY === 0, "a page without anchor did not start at the top");
                top(test).flickable.contentY = 100;
                test.check(!top(test).anchorOwed, "scrolling did not stand down the anchor snap");
                test.shot("10g-docs-code");
            }
        },
        {
            name: "documentation notes",
            run: () => test.app.showDoc("maintenance.md", "cleanup"),
            until: () => guideShown(test, "maintenance.md"),
            delay: 800,
            then: () => {
                const notes = blocksOf(test, "note");
                test.check(notes.length > 0, "maintenance.md shows no note");
                const warning = notes.find(note => note.item.warning);
                test.check(warning !== undefined, "maintenance.md shows no warning");
                test.check(Qt.colorEqual(warning.item.accent, test.themeColor("neutralTextColor")), "a warning is not in the theme's warning colour");
                test.check(top(test).flickable.contentY > 0, "the link to #cleanup did not scroll");
                test.shot("10h-docs-anchor");
            }
        },
        {
            name: "documentation links",
            run: () => top(test).follow("applying.md#automatic-commits"),
            until: () => guideShown(test, "applying.md"),
            delay: 800,
            then: () => {
                test.check(top(test).anchor === "automatic-commits", "the link lost its heading");
                test.check(Helpers.guideLink("https://example.org", "x.md") === null, "a web link is taken for a guide page");
                const same = Helpers.guideLink("#git", "applying.md");
                test.check(same.file === "applying.md" && same.anchor === "git", "a heading link left the page");
                test.check(test.app.pageStack.depth === 2, "a link opened another column: depth " + test.app.pageStack.depth);
            }
        },
        {
            name: "help topics resolve",
            run: () => {
                for (const section of ["overview", "options", "tree", "configured", "home", "packages", "functions", "generations", "actions", "cleanup", "inputs", "impermanence", "history", "changes", "log", "settings", "about"]) {
                    const link = Helpers.guideLink(Helpers.helpTopic(section), "");
                    test.check(link !== null, "section " + section + " has no guide page");
                    const page = test.app.docsModel.page(link.file);
                    test.check(page.error === "", section + ": " + page.error);
                    test.check(link.anchor === "" || page.blocks.some(block => block.anchor === link.anchor), section + ": " + link.file + " has no #" + link.anchor);
                }

                const pages = Results.groups(test.app, "handbook").find(group => group.key === "pages");
                test.check(pages && pages.items.some(item => item.open.section === "docs"), "the Overview search does not find the Documentation");
            }
        },
        {
            name: "help button",
            run: () => {
                test.app.showSection("tree");
                test.app.showSection("overview");
                test.app.showSection("tree");
                const help = helpActions(test.app.sectionPages.tree);
                test.check(help.length === 1, "the Tree View has " + help.length + " Help buttons");
                help[0].trigger();
            },
            until: () => guideShown(test, "options.md"),
            delay: 800,
            then: () => {
                test.check(test.app.section === "docs", "Help did not switch to the Documentation section");
                test.check(top(test).anchor === "browse-all-options-as-a-tree", "Help opened " + top(test).anchor);
                test.check(top(test).flickable.contentY > 0, "Help did not scroll to its heading");
                test.shot("10i-docs-help");
            }
        },
        {
            name: "help menu entry",
            run: () => {
                test.app.showSection("inputs");
                test.app.menuBar.helpAction.trigger();
            },
            until: () => guideShown(test, "inputs.md"),
            delay: 500,
            then: () => test.check(helpOf(test.app.sectionPages.docs) === null, "the Documentation page has a Help button for itself")
        },
        {
            name: "documentation missing page",
            run: () => test.app.showDoc("no-such-page.md", ""),
            until: () => top(test).file === "no-such-page.md" && !top(test).settling,
            delay: 500,
            then: () => {
                test.check(top(test).guide.error !== "", "a missing page shows no error");
                test.check(top(test).header.visible, "the error banner is hidden");
                test.check(test.app.docsModel.page("../Cargo.toml").error !== "", "a path outside the guide was read");
                test.shot("10j-docs-missing");
            }
        },
        {
            name: "documentation back",
            run: () => test.app.showSection("overview"),
            delay: 500
        }
    ];
}
