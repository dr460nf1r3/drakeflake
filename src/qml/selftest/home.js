.pragma library

function steps(test) {
    const noHomeUsers = () => test.app.systemModel.home_users.length === 0;

    return [
        {
            name: "home manager not set up",
            skip: () => !noHomeUsers(),
            run: () => test.app.showSection("home"),
            timeout: 600000,
            until: () => test.app.systemModel.home_state !== "",
            then: () => {
                test.check(test.app.systemModel.home_state === "none", "home state is " + test.app.systemModel.home_state);
                test.shot("09d-home-manager-none");
            },
            delay: 500
        },
        {
            name: "home manager (only when the flake uses it)",
            skip: noHomeUsers,
            run: () => test.app.showSection("home"),
            timeout: 600000,
            until: () => test.app.homeModel.size > 0,
            then: () => {
                test.check(test.app.systemModel.home_state === "available", "home state is " + test.app.systemModel.home_state);
                test.log("home-manager users " + test.app.systemModel.home_users.join(", ") + ", options " + test.app.homeModel.size);
                test.app.homeModel.search("git enable");
                test.app.showOption("home-manager.users.alice.programs.git.enable", test.app.homeModel);
            },
            delay: 2000
        },
        {
            name: "home manager user picker",
            skip: noHomeUsers,
            run: () => {
                const system = test.app.systemModel;
                const picker = test.findChild(test.app.sectionPages.home.header, "userPicker");
                test.check(picker !== null, "the Home Manager page has no user picker");
                test.check(picker.visible === (system.home_users.length > 1), "the user picker shows for " + system.home_users.length + " users");
                test.check(picker.options.length === system.home_users.length, "the user picker lists " + picker.options.length + " users");
                test.check(picker.currentValue === system.home_user && picker.currentLabel === system.home_user, "the user picker shows " + picker.currentLabel);

                const last = system.home_users[system.home_users.length - 1];
                picker.picked(last);
                test.check(system.home_user === last, "picking " + last + " selected " + system.home_user);
                picker.picked(system.home_users[0]);
                test.check(system.home_user === system.home_users[0], "picking the first user selected " + system.home_user);
            }
        },
        {
            name: "screenshot home manager",
            skip: noHomeUsers,
            run: () => {
                const info = test.app.homeModel.details("home-manager.users.alice.programs.git.enable");
                test.log("hm git.enable = " + info.valueDisplay + " (" + info.valueState + ")");
                test.check(info.valueDisplay === "true", "home-manager value missing");
                test.check(test.app.changeModel.stage_option("home-manager.users.alice.programs.git.enable", "boolean", "false") === "", "staging a home-manager option failed");
                test.shot("09d-home-manager");
            }
        }
    ];
}
