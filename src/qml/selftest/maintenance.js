.pragma library


function steps(test) {
    return [
        {
            name: "recommendations",
            run: () => test.app.showSection("actions"),
            timeout: 120000,
            until: () => {
                const page = test.app.pageStack.items[0];
                return !!page.recommendations && page.recommendations.some(item => item.key === "optimise");
            },
            delay: 800
        },
        {
            name: "recommendations listed",
            timeout: 120000,
            until: () => {
                const keys = test.app.pageStack.items[0].recommendations.map(item => item.key);
                return keys.indexOf("nh") >= 0 && keys.indexOf("optimise") >= 0;
            },
            then: () => {
                const keys = test.app.pageStack.items[0].recommendations.map(item => item.key);
                test.log("recommendations: " + keys.join(", "));
                const optimise = test.details("nix.settings.auto-optimise-store");
                test.log("auto-optimise-store: " + JSON.stringify({
                    state: optimise.valueState,
                    display: optimise.valueDisplay,
                    name: optimise.name
                }));
                test.check(keys.indexOf("nh") >= 0 && keys.indexOf("optimise") >= 0, "expected nh and store deduplication recommendations");
                test.check(keys.indexOf("clean") < 0, "nh's cleaner recommended without nh");
                test.shot("09k-recommendations");
            },
            delay: 500
        },
        {
            name: "generations page",
            run: () => {
                test.app.showSection("generations");
                test.app.generationsModel.load_generations();
            },
            until: () => !test.app.generationsModel.generations_loading,
            then: () => {
                const generations = test.app.generationsModel.generations;
                test.log("generations: " + generations.length);
            },
            delay: 800
        },
        {
            name: "screenshot generations",
            run: () => test.shot("09g-generations")
        },
        {
            name: "actions page",
            run: () => test.app.showSection("actions"),
            timeout: 300000,
            until: () => !test.app.imagesModel.loading && (test.app.systemModel.capabilities.images ? (test.app.imagesModel.formats.length > 0 || test.app.imagesModel.error !== "") : true),
            then: () => {
                if (test.app.systemModel.capabilities.images) {
                    const formats = test.app.imagesModel.formats;
                    test.log("image formats: " + formats.length);
                    test.check(formats.indexOf("iso") >= 0, "no iso image format");
                } else {
                    test.check(test.app.imagesModel.formats.length === 0, "formats listed without images");
                }
            },
            delay: 800
        },
        {
            name: "screenshot actions",
            run: () => test.shot("09j-actions")
        },
        {
            name: "cleanup page",
            run: () => {
                test.app.showSection("cleanup");
                test.app.generationsModel.load_cleanup_info();
            },
            until: () => !test.app.generationsModel.cleanup_loading,
            then: () => {
                const info = test.app.generationsModel.cleanup_info;
                test.check(info.total > 0, "no disk usage");
            },
            delay: 800
        },
        {
            name: "screenshot cleanup",
            run: () => test.shot("09h-cleanup")
        },
        {
            name: "compare a generation",
            run: () => {
                const generations = test.app.generationsModel.generations;
                const other = generations.find(generation => !generation.running);
                if (other)
                    test.app.systemModel.generation_action(other.id, "compare");
            },
            timeout: 120000,
            until: () => !test.app.systemModel.running,
            delay: 1000
        },
        {
            name: "show console",
            run: () => test.app.showSection("log"),
            delay: 800
        },
        {
            name: "screenshot console",
            run: () => test.shot("09i-console-compare")
        }
    ];
}
