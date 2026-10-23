.pragma library


function steps(test) {
    return [
        {
            name: "missing flake reported",
            run: () => test.app.showSection("options"),
            until: () => test.app.systemModel.target_error.length > 0,
            timeout: 120000,
            then: () => {
                test.check(!test.app.systemModel.target_ready, "a missing flake is reported as ready");
                test.check(test.app.systemModel.target_error.indexOf("no-such-flake") >= 0, "the error does not name the missing folder: " + test.app.systemModel.target_error);

                const hint = test.findChild(test.app.sectionPages.options.flickable, "optionsPlaceholder");
                test.check(hint !== null && hint.visible, "the Options page shows no placeholder for the missing flake");
                test.check(hint.explanation.indexOf("no-such-flake") >= 0, "the placeholder does not name the missing folder: " + hint.explanation);
            }
        },
        {
            name: "screenshot missing flake",
            run: () => test.shot("no-system"),
            delay: 800
        },
        {
            name: "sections still open",
            run: () => {
                test.app.showSection("settings");
                test.check(test.app.section === "settings", "the settings did not open without a flake");
                test.app.showSection("overview");
            },
            delay: 300
        }
    ];
}
