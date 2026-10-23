.pragma library
.import "../../main/helpers.js" as Helpers

function list(options, changes, darwin, generations, inputs) {
    const value = name => options.details(name).valueDisplay;
    const unstaged = name => changes.option_state(name) === "";
    const found = [];
    const undocumented = options.undocumented();

    // Fixed docs let the app build the option index from the manual whatever the config says.
    if (undocumented.length > 0)
        found.push({
            key: "fix-docs",
            title: qsTranslate("ActionsPage", "Fix the documentation of %1 of your options").arg(undocumented.length),
            text: qsTranslate("ActionsPage", "Their descriptions fail to evaluate, often because of the removed lib.mdDoc (use a plain string instead), so the option index is built the slow way, option by option. First ones: %1").arg(undocumented.slice(0, 5).join(", ")),
            search: undocumented[0]
        });

    if (value("documentation.nixos.enable") === "false" && unstaged("documentation.nixos.enable"))
        found.push({
            key: "manual",
            title: qsTranslate("ActionsPage", "Turn on the NixOS manual"),
            text: undocumented.length > 0 ? qsTranslate("ActionsPage", "Adds man configuration.nix and nixos-help to your system. Fix the option documentation above first: until then the manual cannot be built and the rebuild fails.") : qsTranslate("ActionsPage", "Adds man configuration.nix and nixos-help to your system. Each rebuild renders the manual (a few seconds)."),
            option: "documentation.nixos.enable",
            input: "true"
        });

    if (value("programs.nh.enable") === "false" && unstaged("programs.nh.enable"))
        found.push({
            key: "nh",
            title: qsTranslate("ActionsPage", "Install nh"),
            text: qsTranslate("ActionsPage", "Review and apply, switching between generations and Cleanup use nh: it shows what changes before switching and asks first."),
            option: "programs.nh.enable",
            input: "true"
        });

    // nix-darwin refuses auto-optimise-store ("known to corrupt the Nix
    // Store") and optimises on a schedule instead.
    if (darwin && value("nix.optimise.automatic") === "false" && unstaged("nix.optimise.automatic"))
        found.push({
            key: "optimise",
            title: qsTranslate("ActionsPage", "Deduplicate the Nix store regularly"),
            text: qsTranslate("ActionsPage", "Identical files in the store are stored once (hard links), on a schedule. Saves a lot of disk space."),
            option: "nix.optimise.automatic",
            input: "true"
        });

    if (!darwin && value("nix.settings.auto-optimise-store") === "false" && unstaged("nix.settings.auto-optimise-store"))
        found.push({
            key: "optimise",
            title: qsTranslate("ActionsPage", "Deduplicate the Nix store automatically"),
            text: qsTranslate("ActionsPage", "Identical files in the store are stored once (hard links). Saves a lot of disk space; costs a little time when building."),
            option: "nix.settings.auto-optimise-store",
            input: "true"
        });

    if (value("programs.nh.enable") === "true" && value("nix.gc.automatic") === "false" && value("programs.nh.clean.enable") === "false" && unstaged("programs.nh.clean.enable")) {
        const item = {
            key: "clean",
            title: qsTranslate("ActionsPage", "Clean up old generations automatically"),
            text: qsTranslate("ActionsPage", "Without it, every rebuild keeps its generation and the store keeps growing. nh's cleaner removes old ones weekly (it keeps the recent ones)."),
            option: "programs.nh.clean.enable",
            input: "true"
        };

        // nix-darwin refuses to activate with the cleaner enabled but no
        // user set for it, so the primary user goes along where it is known.
        const extra = cleanUser(options, changes, darwin);
        if (extra)
            item.also = [extra];
        found.push(item);
    }

    if (value("programs.nh.enable") === undefined && value("nix.gc.automatic") === "false" && unstaged("nix.gc.automatic"))
        found.push({
            key: "gc",
            title: qsTranslate("ActionsPage", "Clean up old generations automatically"),
            text: qsTranslate("ActionsPage", "Without it, every rebuild keeps its generation and the store keeps growing. Nix's garbage collector then removes old ones on a schedule."),
            option: "nix.gc.automatic",
            input: "true"
        });

    if (generations.length > 20)
        found.push({
            key: "generations",
            title: qsTranslate("ActionsPage", "%1 system generations are kept").arg(generations.length),
            text: qsTranslate("ActionsPage", "Old generations keep their packages on disk. Remove the old ones in Cleanup."),
            section: "cleanup"
        });

    const nixpkgs = inputs.find(input => input.name === "nixpkgs");
    const days = nixpkgs && nixpkgs.last_modified ? Helpers.ageInDays(nixpkgs.last_modified) : 0;
    if (days > 30)
        found.push({
            key: "inputs",
            title: qsTranslate("ActionsPage", "nixpkgs is %1 days old").arg(days),
            text: qsTranslate("ActionsPage", "Updating brings security fixes and newer packages. Check your inputs."),
            section: "inputs"
        });

    return found;
}

function cleanUser(options, changes, darwin) {
    if (!darwin)
        return null;

    const user = options.details("system.primaryUser").valueInput;

    if (typeof user !== "string" || user === "")
        return null;

    const configured = options.details("programs.nh.clean.user");

    if (typeof configured.type !== "string" || configured.type === "")
        return null;

    const set = configured.valueInput;

    if (changes.option_state("programs.nh.clean.user") !== "")
        return null;

    if (set !== undefined && set !== null && set !== "null")
        return null;

    return {
        option: "programs.nh.clean.user",
        input: user,
        type: configured.type
    };
}
