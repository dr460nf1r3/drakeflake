.pragma library
.import "../../main/helpers.js" as Helpers
.import "entries.js" as Entries

const PER_GROUP = 3;

function words(query) {
    return query.toLowerCase().split(/\s+/).filter(word => word.length > 0);
}

function matches(query, text) {
    const haystack = text.toLowerCase();
    return words(query).every(word => haystack.indexOf(word) >= 0);
}

function filterEntries(entries, query) {
    const first = words(query)[0] || "";

    return entries.filter(entry => matches(query, [entry.title, entry.subtitle, entry.keywords].join(" "))).map((entry, position) => ({
                entry: entry,
                rank: entry.title.toLowerCase().startsWith(first) ? 0 : 1,
                position: position
            })).sort((a, b) => a.rank - b.rank || a.position - b.position).map(ranked => ranked.entry);
}

function group(key, title, items, total, more, moreOpen) {
    return {
        key: key,
        title: title,
        total: total,
        more: total > items.length && more ? more : "",
        moreOpen: moreOpen || null,
        items: items.slice(0, PER_GROUP)
    };
}

function entryGroup(key, title, entries, query) {
    const found = filterEntries(entries, query);
    const items = found.map(entry => ({
                title: entry.title,
                subtitle: entry.subtitle,
                trailing: "",
                mono: false,
                open: entry.open
            }));

    return group(key, title, items, found.length, "", null);
}

function optionGroup(key, title, more, model, query, kind, strip) {
    const found = model.top(query, PER_GROUP);
    const items = found.rows.map(row => ({
                title: strip(row.name),
                subtitle: kind === "configured" && row.defined_in ? qsTr("Set in %1").arg(row.defined_in) : row.summary,
                trailing: row.value,
                mono: true,
                open: {
                    kind: kind,
                    name: row.name
                }
            }));

    return group(key, title, items, found.total, more.arg(Helpers.count(found.total)), {
        kind: "search",
        section: key,
        query: query
    });
}

function packageGroup(app, query) {
    const found = app.packageModel.top(query, PER_GROUP);
    const items = found.rows.map(row => ({
                title: row.attr,
                subtitle: row.description,
                trailing: row.installed ? qsTr("installed") : "",
                mono: false,
                open: {
                    kind: "package",
                    attr: row.attr
                }
            }));

    return group("packages", qsTr("Packages"), items, found.total, qsTr("All %1 in Packages").arg(Helpers.count(found.total)), {
        kind: "search",
        section: "packages",
        query: query
    });
}

function functionGroup(app, query) {
    const found = app.functionModel.top(query, PER_GROUP);
    const items = found.rows.map(row => ({
                title: row.path,
                subtitle: row.signature.length > 0 ? row.signature : row.summary,
                trailing: row.builtin ? qsTr("builtin") : "",
                mono: true,
                open: {
                    kind: "function",
                    path: row.path
                }
            }));

    return group("functions", qsTr("Functions"), items, found.total, qsTr("All %1 in Functions").arg(Helpers.count(found.total)), {
        kind: "search",
        section: "functions",
        query: query
    });
}

function sourceGroup(app, query) {
    const found = app.searchModel.flake_hits;

    if (found.query !== query.trim() || found.hits.length === 0)
        return null;

    const items = found.hits.map(hit => {
        const place = hit.display + ":" + hit.line;
        const attribute = hit.kind === "attribute";

        return {
            title: attribute ? hit.path : hit.snippet,
            subtitle: attribute ? [place, hit.snippet].filter(part => part.length > 0).join(" · ") : [hit.path, place].filter(part => part.length > 0).join(" · "),
            trailing: "",
            mono: true,
            open: {
                kind: "source",
                file: hit.file,
                line: hit.line
            }
        };
    });

    return group("source", qsTr("In your flake's files"), items, found.total, "", null);
}

function commandGroup(app, query) {
    const commands = app.packageModel.overview_commands;

    if (commands.command !== query.trim() || commands.attrs.length === 0)
        return null;

    const items = commands.attrs.map(attr => ({
                title: commands.command,
                subtitle: qsTr("Program in %1").arg(attr),
                trailing: "",
                mono: true,
                open: {
                    kind: "package",
                    attr: attr
                }
            }));

    return group("commands", qsTr("Programs"), items, items.length, "", null);
}

function listGroup(key, title, rows, query, haystack, item) {
    const found = rows.filter(row => matches(query, haystack(row)));
    const section = {
        kind: "section",
        section: key
    };

    return group(key, title, found.map(row => Object.assign(item(row), {
            open: section
        })), found.length, qsTr("Show all"), section);
}

function inputGroup(app, query) {
    return listGroup("inputs", qsTr("Inputs"), app.inputsModel.inputs, query, input => [input.name, input.source].join(" "), input => ({
                title: input.name,
                subtitle: input.source,
                trailing: input.check && input.check.state === "available" ? qsTr("update available") : "",
                mono: false
            }));
}

function generationGroup(app, query) {
    const details = generation => [Helpers.formatDate(generation.date), generation.version, generation.kernel].filter(part => part && part.length > 0).join(" · ");

    return listGroup("generations", qsTr("Generations"), app.generationsModel.generations, query, generation => [generation.id, details(generation)].join(" "), generation => ({
                title: qsTr("Generation %1").arg(generation.id),
                subtitle: details(generation),
                trailing: generation.running ? qsTr("running") : "",
                mono: false
            }));
}

function historyGroup(app, query) {
    const when = entry => entry.saved ? qsTr("Replaced %1").arg(Helpers.formatDate(entry.saved)) : qsTr("Earlier settings");

    return listGroup("history", qsTr("History"), app.generationsModel.history, query, entry => [when(entry)].concat(entry.changes).join(" "), entry => ({
                title: when(entry),
                subtitle: entry.changes.slice(0, 3).join(", "),
                trailing: "",
                mono: false
            }));
}

function groups(app, query) {
    const can = app.systemModel.capabilities;
    const offered = entry => Helpers.sectionOffered(entry.open.section, app.macos) && (can.generations || entry.open.section !== "generations") && (can.images || !entry.open.images) && (entry.open.kind !== "rebuild" || can.rebuild.includes(entry.open.mode));
    const pages = Entries.pages().filter(offered);
    const actions = Entries.actions().filter(offered);
    const optionsTitle = app.systemModel.kind === "darwin" ? qsTr("nix-darwin options") : qsTr("NixOS options");
    const homePrefix = name => name.split(".").slice(3).join(".");
    const all = [entryGroup("pages", qsTr("Pages"), pages, query), entryGroup("actions", qsTr("Actions"), actions, query), entryGroup("settings", qsTr("Settings"), Entries.settings(), query), optionGroup("options", optionsTitle, qsTr("All %1 in Options"), app.optionModel, query, "option", name => name), optionGroup("configured", qsTr("Set in your flake"), qsTr("All %1 in Configured"), app.configuredModel, query, "configured", name => name)];
    const source = sourceGroup(app, query);

    if (source)
        all.push(source);

    if (app.hasHomeManager && app.homeModel.size > 0)
        all.push(optionGroup("home", qsTr("Home Manager · %1").arg(app.systemModel.home_user), qsTr("All %1 in Home Manager"), app.homeModel, query, "home", homePrefix));

    all.push(packageGroup(app, query));

    const commands = commandGroup(app, query);

    if (commands)
        all.push(commands);

    all.push(functionGroup(app, query), inputGroup(app, query), generationGroup(app, query), historyGroup(app, query));
    return all;
}

function rows(groups) {
    const result = [];

    for (const found of groups.filter(group => group.items.length > 0)) {
        result.push({
            header: true,
            group: found
        });

        for (const item of found.items)
            result.push({
                header: false,
                item: item
            });
    }

    return result;
}
