.pragma library
.import "../pages/overview/results.js" as Results


function hits(groups, key) {
    const found = groups.find(group => group.key === key);
    return found ? found.items.length : 0;
}

function steps(test) {
    return [
        {
            name: "overview page",
            run: () => test.app.showSection("overview"),
            delay: 1000
        },
        {
            name: "overview search",
            run: () => {
                test.shot("01b-overview");
                const ssh = Results.groups(test.app, "openssh");
                test.check(hits(ssh, "options") > 0, "overview search: no options for openssh");
                test.check(hits(ssh, "packages") > 0, "overview search: no packages for openssh");
                test.check(hits(Results.groups(test.app, "inputs"), "pages") > 0, "overview search: the Inputs page is not found");

                if (test.app.systemModel.capabilities.rebuild.length > 0)
                    test.check(hits(Results.groups(test.app, "rebuild"), "actions") > 0, "overview search: no rebuild actions");
                test.log("overview search: " + Results.rows(ssh).length + " rows for openssh");
            }
        },
        {
            name: "overview dropdown",
            run: () => test.app.pageStack.items[0].search.searchFor("ssh"),
            delay: 1500
        },
        {
            name: "screenshot overview dropdown",
            run: () => {
                test.check(test.app.pageStack.items[0].search.rows.length > 0, "overview dropdown is empty");
                test.shot("01c-overview-search");
            },
            delay: 1000
        },
        {
            name: "overview search cleared",
            run: () => test.app.pageStack.items[0].search.searchFor("")
        }
    ];
}

function flakeSource(test) {
    const search = () => test.app.pageStack.items[0].search;
    const answered = query => test.app.searchModel.flake_hits.query === query && !test.app.searchModel.searching;
    const sourceItems = () => {
        const found = search().groups.find(group => group.key === "source");
        return found ? found.items : [];
    };

    return [
        {
            name: "flake search: overview",
            run: () => test.app.showSection("overview"),
            delay: 800
        },
        {
            name: "flake search: attribute",
            run: () => search().searchFor("hostname"),
            timeout: 30000,
            until: () => answered("hostname"),
            then: () => {
                const hit = sourceItems().find(item => item.title === "networking.hostName" && item.subtitle.indexOf("configuration.nix:") === 0);
                test.check(hit !== undefined, "networking.hostName not found in configuration.nix: " + JSON.stringify(sourceItems()));
                test.check(hit.open.kind === "source" && hit.open.file.endsWith("/configuration.nix") && hit.open.line > 1, "hit opens " + JSON.stringify(hit.open));
            }
        },
        {
            name: "screenshot flake search",
            run: () => test.shot("01d-overview-flake-search"),
            delay: 800
        },
        {
            name: "flake search: value in a list",
            run: () => search().searchFor("pkgs.git"),
            timeout: 30000,
            until: () => answered("pkgs.git"),
            then: () => {
                const hit = test.app.searchModel.flake_hits.hits.find(found => found.kind === "value" && found.path === "environment.systemPackages");
                test.check(hit !== undefined, "pkgs.git not found as a value: " + JSON.stringify(test.app.searchModel.flake_hits));
            }
        },
        {
            name: "flake search: no hits",
            run: () => search().searchFor("zz-no-such-word"),
            timeout: 30000,
            until: () => answered("zz-no-such-word"),
            then: () => {
                test.check(test.app.searchModel.flake_hits.total === 0, "hits for nonsense: " + JSON.stringify(test.app.searchModel.flake_hits));
                test.check(sourceItems().length === 0, "source group shown without hits");
                search().searchFor("");
            }
        }
    ];
}
