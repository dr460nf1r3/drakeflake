.pragma library

function steps(test) {
    return [
        {
            name: "menu bar",
            run: () => {
                const bar = test.app.menuBar;
                test.check(bar !== null, "no menu bar on the window");

                const titles = [];
                for (let i = 0; i < bar.menus.length; i++)
                    titles.push(bar.menus[i].title);

                for (const wanted of ["File", "Edit", "View", "Window", "Help"])
                    test.check(titles.indexOf(wanted) >= 0, "menu bar misses " + wanted + ": " + JSON.stringify(titles));

                const about = bar.aboutAction;
                test.check(about !== undefined && about !== null, "no About action in the menu bar");
                about.trigger();
                test.check(test.app.section === "about", "the About menu entry did not open the About section");
            },
            then: () => test.shot(test.isDarwin ? "darwin-menu" : "10d-menu"),
            delay: 800
        },
        {
            name: "menu bar back",
            run: () => test.app.showSection("overview"),
            delay: 800
        },
        {
            // Qt.labs.platform menus crash the offscreen platform, so the mirror itself is not built here.
            name: "menu bar without a global menu",
            run: () => {
                const bar = test.app.menuBar;
                test.check(bar.visible === !bar.exported, "the in-window menu bar shows next to a global menu");
                test.check(bar.hiddenForShot === false, "a screenshot left the menu bar hidden");
            }
        },
        {
            name: "sidebar groups",
            run: () => {
                const drawer = test.app.globalDrawer;
                test.check(drawer !== null, "no drawer on the window");

                const rows = [];
                for (let i = 0; i < drawer.actions.length; i++) {
                    const action = drawer.actions[i];
                    rows.push(action.section !== undefined ? action.section : (action.separator ? "-" : "?"));
                }

                const wanted = ["overview", "options", "home", "packages", "functions", "generations", "tree", "configured", "-", "actions", "cleanup", "inputs", "impermanence", "history", "-", "changes", "log", "-", "settings", "docs", "about"];
                test.check(JSON.stringify(rows) === JSON.stringify(wanted), "sidebar order changed: " + JSON.stringify(rows));

                const overview = drawer.leaf("overview");
                test.check(overview !== null, "no Overview leaf in the sidebar");
                overview.trigger();
                test.check(test.app.section === "overview", "sidebar Overview did not open its section");
            }
        },
        {
            name: "shortcut find",
            run: () => {
                test.app.showSection("options");
                test.app.menuBar.findAction.trigger();

                const focused = test.app.activeFocusItem;
                test.check(focused !== null && focused.placeholderText !== undefined && focused.placeholderText.indexOf("Search") === 0, "Find did not focus the search field");
            },
            delay: 500
        },
        {
            name: "shortcut back",
            run: () => {
                test.app.showSection("options");
                test.app.showOption("services.openssh.enable");
                test.check(test.app.pageStack.depth === 2, "option details did not push");

                test.app.menuBar.backAction.trigger();
                test.check(test.app.pageStack.depth === 1, "Back did not pop the details");

                test.app.showSection("overview");
            },
            delay: 500
        },
        {
            name: "field menu select all",
            run: () => {
                test.app.showSection("options");
                const fieldMenu = test.findChild(test.app.contentItem, "optionsFieldMenu");
                test.check(fieldMenu !== null, "no field menu on the options search");
                fieldMenu.target.text = "openssh";
                fieldMenu.selectAllAction.trigger();
                test.check(fieldMenu.target.selectedText === "openssh", "Select All did not select the query: " + JSON.stringify(fieldMenu.target.selectedText));
                fieldMenu.target.text = "";
            }
        },
        {
            name: "save guard",
            run: () => {
                const save = test.app.menuBar.saveAction;
                test.check(save.enabled === (test.app.changeModel.count > 0), "Apply Changes enabled without staged changes");
            }
        }
    ];
}
