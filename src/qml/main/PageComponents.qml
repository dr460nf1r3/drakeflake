pragma ComponentBehavior: Bound

import QtQuick

import "../pages"
import "helpers.js" as Helpers

QtObject {
    id: pages

    required property var app
    readonly property Component options: Component {
        OptionsPage {
            app: pages.app
        }
    }
    readonly property Component tree: Component {
        TreePage {
            app: pages.app
        }
    }
    readonly property Component optionDetails: Component {
        OptionDetailsPage {
            app: pages.app
        }
    }
    readonly property Component configured: Component {
        ConfiguredPage {
            app: pages.app
        }
    }
    readonly property Component home: Component {
        HomeManagerPage {
            app: pages.app
        }
    }
    readonly property Component preview: Component {
        PreviewPage {
            app: pages.app
        }
    }
    readonly property Component packageDetails: Component {
        PackageDetailsPage {
            app: pages.app
        }
    }
    readonly property Component packages: Component {
        PackagesPage {
            app: pages.app
        }
    }
    readonly property Component functions: Component {
        FunctionsPage {
            app: pages.app
        }
    }
    readonly property Component functionDetails: Component {
        FunctionDetailsPage {
            app: pages.app
        }
    }
    readonly property Component changes: Component {
        ChangesPage {
            app: pages.app
        }
    }
    readonly property Component about: Component {
        AboutPage {
            app: pages.app
        }
    }
    readonly property Component generations: Component {
        GenerationsPage {
            app: pages.app
        }
    }
    readonly property Component actions: Component {
        ActionsPage {
            app: pages.app
        }
    }
    readonly property Component cleanup: Component {
        CleanupPage {
            app: pages.app
        }
    }
    readonly property Component inputs: Component {
        InputsPage {
            app: pages.app
        }
    }
    readonly property Component impermanence: Component {
        ImpermanencePage {
            app: pages.app
        }
    }
    readonly property Component history: Component {
        HistoryPage {
            app: pages.app
        }
    }
    readonly property Component overview: Component {
        OverviewPage {
            app: pages.app
        }
    }
    readonly property Component log: Component {
        LogPage {
            app: pages.app
        }
    }
    readonly property Component docs: Component {
        DocsPage {
            app: pages.app
        }
    }
    readonly property Component guide: Component {
        GuidePage {
            app: pages.app
        }
    }
    readonly property Component help: Component {
        HelpAction {
            app: pages.app
        }
    }
    readonly property Component settings: Component {
        SettingsPage {
            app: pages.app
        }
    }

    function addHelp(page: var, section: string): void {
        if (Helpers.helpTopic(section) !== "")
            page.actions.push(pages.help.createObject(page, {
                section: section
            }));
    }

    function forSection(name: string): Component {
        switch (name) {
        case "overview":
            return pages.overview;
        case "options":
            return pages.options;
        case "tree":
            return pages.tree;
        case "configured":
            return pages.configured;
        case "home":
            return pages.home;
        case "packages":
            return pages.packages;
        case "functions":
            return pages.functions;
        case "generations":
            return pages.generations;
        case "actions":
            return pages.actions;
        case "cleanup":
            return pages.cleanup;
        case "inputs":
            return pages.inputs;
        case "impermanence":
            return pages.impermanence;
        case "changes":
            return pages.changes;
        case "history":
            return pages.history;
        case "log":
            return pages.log;
        case "settings":
            return pages.settings;
        case "about":
            return pages.about;
        case "docs":
            return pages.docs;
        default:
            return null;
        }
    }
}
