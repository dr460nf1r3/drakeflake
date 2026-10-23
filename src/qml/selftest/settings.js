.pragma library


function steps(test) {
    return [
        {
            name: "settings page",
            run: () => test.app.showSection("settings"),
            delay: 800
        },
        {
            name: "screenshot settings",
            run: () => test.shot("10-settings")
        },
        {
            name: "log level and file",
            run: () => {
                const settings = test.app.settingsModel;
                test.check(settings.log_level === "info", "default log level is " + settings.log_level);
                test.check(settings.log_file().indexOf("/state/drakeflake/drakeflake.log") >= 0, "log file at " + settings.log_file());
                settings.set_log_level("debug");
                test.check(settings.log_level === "debug", "log level not changed");
                settings.set_log_level("loud");
                test.check(settings.log_level === "debug", "unknown log level accepted");
                settings.set_log_level("info");
            }
        },
        {
            name: "automatic commit setting",
            run: () => {
                const settings = test.app.settingsModel;
                test.check(settings.auto_commit, "automatic commits are off by default");
                settings.set_auto_commit(false);
                test.check(!settings.auto_commit, "automatic commits not switched off");

                settings.set_auto_commit(false);
                test.check(!settings.auto_commit, "switching off twice turned it on");

                settings.set_auto_commit(true);
                test.check(settings.auto_commit, "automatic commits not switched back on");
            }
        },
        {
            name: "tray settings",
            run: () => {
                const settings = test.app.settingsModel;
                const tray = () => settings.tray_settings;
                test.check(tray().updates && tray().low_space && tray().check_hours === 6, "tray defaults: " + JSON.stringify(tray()));
                settings.set_tray_report("low_space", false);
                test.check(tray().low_space === false, "low_space not switched off");
                test.check(settings.set_tray_interval("check_hours", 12) === "", "12 hours refused");
                test.check(tray().check_hours === 12, "check_hours not stored");

                test.check(settings.set_tray_interval("health_minutes", 0) !== "", "0 minutes accepted");
                test.check(tray().health_minutes === 1, "health_minutes changed to " + tray().health_minutes);

                settings.set_tray_report("low_space", true);
                settings.set_tray_interval("check_hours", 6);

                test.check(tray().watch_flake === false, "flake watch on by default");
                settings.set_tray_report("watch_flake", true);
                test.check(tray().watch_flake === true, "flake watch not switched on");
            },
            then: () => test.shot("10a-settings-tray"),
            delay: 800
        },
        {
            name: "tray flake watch off",
            run: () => {
                test.app.settingsModel.set_tray_report("watch_flake", false);
                test.check(test.app.settingsModel.tray_settings.watch_flake === false, "flake watch not switched off");
            }
        },
        {
            name: "blur setting",
            run: () => {
                const settings = test.app.settingsModel;
                test.check(settings.blur === "auto", "default blur is " + settings.blur);
                test.check(settings.blur_env() === "", "blur env override: " + settings.blur_env());

                test.check(settings.set_blur("sometimes") !== "", "unknown blur mode accepted");
                test.check(settings.blur === "auto", "blur changed to " + settings.blur);
                test.check(settings.set_blur("off") === "", "blur off refused");
                test.check(settings.blur === "off", "blur not stored");

                test.check(settings.blur_effective === false, "blur on without Wayland");
                test.check(!test.app.vibrancy, "vibrancy without the blur");
                settings.set_blur("auto");
                test.check(settings.blur === "auto", "blur not restored");
            }
        },
        {
            name: "highlighting theme",
            run: () => {
                test.app.settingsModel.set_highlight_theme("Monokai");
                test.check(test.app.settingsModel.highlight_theme === "Monokai", "highlight theme not stored");
            },
            delay: 1000
        },
        {
            name: "screenshot highlighting theme",
            run: () => test.shot("10b-settings-theme"),
            delay: 1000
        },
        {
            name: "about page",
            run: () => test.app.showSection("about"),
            delay: 500
        },
        {
            name: "screenshot about",
            run: () => test.shot("10c-about"),
            delay: 500
        }
    ];
}
