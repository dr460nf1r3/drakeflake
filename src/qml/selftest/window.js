.pragma library

function steps(test) {
    return [
        {
            name: "window title",
            run: () => {
                const host = test.app.systemModel.host;
                test.check(host !== "", "no host for the window title");
                test.check(test.app.title.indexOf(host) >= 0, "title misses the host: " + test.app.title);
                test.check(test.app.dirty === (test.app.changeModel.count > 0), "dirty flag misses the staged changes");
            }
        },
        {
            name: "window geometry",
            run: () => {
                test.app.settingsModel.save_window_geometry(30, 40, 800, 600);
                const saved = test.app.settingsModel.window_geometry;
                test.check(saved.x === 30, "saved x lost: " + JSON.stringify(saved));
                test.check(saved.y === 40, "saved y lost: " + JSON.stringify(saved));
                test.check(saved.width === 800, "saved width lost: " + JSON.stringify(saved));
                test.check(saved.height === 600, "saved height lost: " + JSON.stringify(saved));

                test.app.width = 500;
                test.app.height = 400;
                test.app.windowKeeper.restore();
                test.check(test.app.width === 800 && test.app.height === 600, "restore missed: " + test.app.width + "x" + test.app.height);
            }
        },
        {
            name: "accessibility flags",
            run: () => {
                test.check(typeof test.app.reduceTransparency === "boolean", "no transparency flag on the window");
                test.check(typeof test.app.reduceMotion === "boolean", "no motion flag on the window");
            }
        },
        {
            name: "window fullscreen",
            run: () => {
                test.app.menuBar.fullscreenAction.trigger();
                test.check(test.app.fullscreen, "fullscreen did not engage");
                test.app.menuBar.fullscreenAction.trigger();
                test.check(!test.app.fullscreen, "fullscreen did not leave");
            },
            then: () => test.shot(test.isDarwin ? "darwin-window" : "11-window"),
            delay: 800
        },
        {
            name: "window size restored",
            run: () => test.restoreWindowSize(),
            until: () => test.app.width === 1600 && test.app.height === 900,
            delay: 800
        }
    ];
}
