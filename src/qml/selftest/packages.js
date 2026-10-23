.pragma library


function search(test) {
    return [
        {
            name: "packages index",
            run: () => {
                test.app.showSection("packages");
                test.app.packageQuery = "firefox";
                test.app.packageModel.search("firefox");
            },
            timeout: 600000,
            until: () => test.app.packageModel.size > 0 && test.app.packageModel.total > 0
        },
        {
            name: "screenshot packages",
            run: () => {
                test.log("packages: " + test.app.packageModel.size);
                test.shot("07-packages");
            },
            delay: 800
        },
        {
            name: "packages listed by default",
            run: () => {
                test.app.packageModel.search("");
                test.check(test.app.packageModel.total === test.app.packageModel.size, "empty query lists " + test.app.packageModel.total + " of " + test.app.packageModel.size);
                test.app.packageModel.search("firefox");
            }
        },
        {
            name: "package details",
            run: () => test.app.showPackage("firefox"),
            timeout: 120000,
            until: () => test.app.packageModel.details("firefox").state === "ready",
            then: () => {
                const info = test.app.packageModel.details("firefox");
                test.log("firefox: " + info.meta.mainProgram + " " + (info.meta.licenses || []).join(","));
                test.check(info.meta.mainProgram === "firefox", "firefox metadata missing");
            },
            delay: 800
        },
        {
            name: "screenshot package details",
            run: () => test.shot("07b-package-details")
        },
        {
            name: "stage packages",
            run: () => {
                const c = test.app.changeModel;
                test.check(c.stage_package("hello", true) === "", "package stage failed");
                test.check(c.stage_package("rm -rf /", true) !== "", "bad package accepted");
                test.check(c.count === 3, "expected 3 staged, got " + c.count);
                test.check(c.package_state("hello") === "add", "hello not staged");
            },
            delay: 500
        },
        {
            name: "screenshot staged package",
            run: () => test.shot("08-packages-staged")
        }
    ];
}

function sets(test) {
    return [
        {
            name: "package sets listed",
            run: () => test.app.showSection("packages"),
            timeout: 300000,
            until: () => test.app.packageModel.package_sets().length > 0,
            then: () => {
                const sets = test.app.packageModel.package_sets();
                test.log("package sets: " + sets.length);
                test.check(sets.indexOf("vimPlugins") >= 0, "vimPlugins set missing");
                test.app.packageModel.set_package_set("vimPlugins");
            }
        },
        {
            name: "package set indexed",
            timeout: 300000,
            until: () => test.app.packageModel.package_set === "vimPlugins" && !test.app.packageModel.set_loading && test.app.packageModel.size > 0,
            then: () => {
                test.check(test.app.packageModel.set_error === "", "set error: " + test.app.packageModel.set_error);
                test.app.packageModel.search("telescope");
                test.check(test.app.packageModel.total > 0, "no telescope plugins found");
                test.log("vimPlugins: " + test.app.packageModel.size + " packages, " + test.app.packageModel.total + " match telescope");
            },
            delay: 800
        },
        {
            name: "screenshot package set",
            run: () => test.shot("08b-package-set")
        },
        {
            name: "open package set picker",
            run: () => {
                const picker = test.findChild(test.app.pageStack.items[0], "packageSetBox");
                test.check(picker !== null, "package set picker not found");
                if (picker) {
                    picker.open();
                    picker.filterText = "python";
                }
            },
            delay: 800
        },
        {
            name: "screenshot package set picker",
            run: () => {
                const shown = test.findChild(test.app.pageStack.items[0], "packageSetBox");
                test.log("picker open: " + (shown ? shown.popupOpen : "missing") + ", filter: " + (shown ? shown.filterText : ""));
                test.shot("08c-package-set-picker");
            },
            delay: 800
        },
        {
            name: "close package set picker",
            run: () => {
                const picker = test.findChild(test.app.pageStack.items[0], "packageSetBox");
                if (picker)
                    picker.close();
            },
            delay: 300
        },
        {
            name: "back to top-level packages",
            run: () => {
                test.app.packageModel.set_package_set("");
                test.app.packageModel.search("");
                test.check(test.app.packageModel.package_set === "", "set not reset");
            }
        }
    ];
}
