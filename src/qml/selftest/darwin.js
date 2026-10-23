.pragma library
.import "../main/kinds.js" as Kinds


function steps(test) {
    return [
        {
            name: "overview while loading",
            run: () => test.app.showSection("overview"),
            delay: 300,
            then: () => test.shot("darwin-overview-loading")
        },
        {
            name: "darwin target",
            timeout: 600000,
            until: () => test.app.systemModel.target_ready || test.app.systemModel.target_error.length > 0,
            then: () => {
                test.check(test.app.systemModel.target_error === "", "target error: " + test.app.systemModel.target_error);
                test.check(test.app.systemModel.kind === "darwin", "kind is " + test.app.systemModel.kind);
                const found = test.app.systemModel.hosts.find(host => host.name === "darwinhost");
                test.check(!!found && found.kind === "darwin", "darwinhost missing: " + JSON.stringify(test.app.systemModel.hosts));
            }
        },
        {
            name: "darwin options",
            run: () => test.app.showOption("networking.hostName"),
            timeout: 1800000,
            until: () => test.valueSettled("networking.hostName"),
            then: () => {
                const info = test.details("networking.hostName");
                test.check(info.valueState === Kinds.valueReady, "hostName not ready: " + info.valueState);
                test.shot("darwin-option-details");
            }
        },
        {
            name: "darwin stage and save",
            run: () => {
                const type = test.details("networking.hostName").type;
                test.check(test.app.changeModel.stage_option("networking.hostName", type, "selftest-mac") === "", "stage failed");
                test.app.startApply(test.applyMode === "" ? "save" : test.applyMode);
            },
            timeout: 1800000,
            until: () => !test.app.systemModel.running && test.app.systemModel.last_result !== "",
            then: () => {
                test.check(test.app.systemModel.last_result === Kinds.outcomeOk, "save failed");
                test.checkCommitted();
            }
        },
        {
            name: "darwin save only here",
            run: () => test.app.showSection("changes"),
            delay: 800,
            then: () => {
                const can = test.app.systemModel.capabilities;
                const onMac = can.platform === "darwin";
                const expected = onMac ? ["save", "build", "switch"] : ["save"];
                test.check(can.native === onMac, "native is " + can.native + " on " + can.platform);
                test.check(can.apply.length === expected.length && expected.every(mode => can.apply.includes(mode)), "apply modes " + JSON.stringify(can.apply));
                test.check(can.generations === onMac, "generations offered: " + can.generations);
                test.check(!can.images, "darwin offers images");
                test.check(test.app.systemModel.rebuild("test") !== "", "darwin accepted a test rebuild");
                test.check(!test.app.systemModel.running, "a refused rebuild started");
                test.shot("darwin-changes");
            }
        },
        {
            name: "darwin recommendations",
            run: () => test.app.showSection("actions"),
            timeout: 600000,
            until: () => {
                const page = test.app.pageStack.items[0];
                return !!page.recommendations && page.recommendations.some(item => item.key === "optimise");
            },
            then: () => {
                const items = test.app.pageStack.items[0].recommendations;
                const optimise = items.find(item => item.key === "optimise");
                test.log("darwin recommendations: " + items.map(item => item.key).join(", "));
                test.check(optimise.option === "nix.optimise.automatic", "darwin optimises with " + optimise.option);
                test.check(items.some(item => item.option === "nix.gc.automatic"), "no nix.gc recommendation");
                test.check(!items.some(item => item.key === "nh" || item.key === "manual"), "NixOS-only recommendation on darwin");
                const onMac = test.app.systemModel.capabilities.platform === "darwin";
                test.check(onMac || !items.some(item => item.key === "generations"), "this Linux machine's generations recommended for a darwin host");
                test.shot("darwin-actions");
            },
            delay: 800
        },
        {
            name: "darwin generations empty",
            run: () => test.app.generationsModel.load_generations(),
            until: () => !test.app.generationsModel.generations_loading,
            then: () => {
                const onMac = test.app.systemModel.capabilities.platform === "darwin";
                test.check(onMac || test.app.generationsModel.generations.length === 0, "darwin lists generations");
            }
        },
        {
            name: "darwin images refused",
            run: () => test.app.imagesModel.load_formats(""),
            until: () => !test.app.imagesModel.loading && test.app.imagesModel.error.length > 0,
            then: () => test.check(test.app.imagesModel.formats.length === 0, "darwin lists image formats")
        },
        {
            name: "platform theme colours",
            then: () => {
                const onMac = test.app.systemModel.capabilities.platform === "darwin";
                const link = test.themeColor("linkColor");
                const breezeLinks = ["#1d99f3", "#2980b9"];

                const desktopTheme = test.app.systemModel.selftest("theme") !== "";
                test.check(desktopTheme || onMac !== breezeLinks.includes(link), "link colour " + link);
                test.check(!onMac || test.themeColor("highlightColor") !== "#3daee9", "Breeze highlight on macOS");
                test.check(!onMac || test.themeColor("negativeTextColor") !== "#da4453", "Breeze error colour on macOS");
                test.check(test.app.pageStack.globalToolBar.separatorVisible === !onMac, "toolbar line shown on macOS or hidden on Linux");

                const offscreen = Qt.platform.pluginName === "offscreen";
                test.check(!offscreen || (!test.app.vibrancy && test.app.color.a === 1), "see-through window without the blur");
                test.check(!offscreen || !test.app.unifiedTitlebar, "unified titlebar without the blur");
            }
        },
        {
            name: "darwin settings badge",
            run: () => test.app.showSection("settings"),
            delay: 800,
            then: () => test.shot("darwin-settings")
        }
    ];
}
