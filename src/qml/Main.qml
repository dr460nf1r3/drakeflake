pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Window
import org.kde.kirigami as Kirigami
import org.kde.syntaxhighlighting

import "components"
import "main"
import "main/helpers.js" as Helpers

Kirigami.ApplicationWindow {
    id: root

    required property var optionModel
    required property var configuredModel
    required property var homeModel
    readonly property bool hasHomeManager: root.systemModel.home_users.length > 0
    required property var packageModel
    required property var changeModel
    required property var systemModel
    // Not `required`: a program with an older API_VERSION must still load far enough to show the rebuild banner.
    property var consoleModel: null
    property var treeModel: null
    property var homeTreeModel: null
    property var setupModel: null
    property var inputsModel: null
    property var imagesModel: null
    property var generationsModel: null
    property var settingsModel: null
    property var functionModel: null
    property var searchModel: null
    property var docsModel: null
    property var persistenceModel: null
    // An older program with newer QML files lacks slots, so this must match API_VERSION (models/system.rs).
    readonly property int expectedApi: 32
    readonly property bool programOutdated: typeof root.systemModel.api_version !== "function" || root.systemModel.api_version() !== root.expectedApi
    readonly property string desktopHighlightTheme: root.matchingHighlightTheme(root.settingsModel.desktop_theme)
    readonly property bool macos: root.systemModel.capabilities.platform === "darwin"
    property bool vibrancy: false
    property bool dirty: root.changeModel.count > 0
    readonly property bool fullscreen: root.visibility === Window.FullScreen
    property bool reduceTransparency: false
    property bool reduceMotion: false
    readonly property alias windowKeeper: windowState
    readonly property alias quitGuard: quitGuard
    readonly property bool unifiedTitlebar: root.macos && root.vibrancy
    readonly property real titlebarHeight: root.unifiedTitlebar ? 28 : 0
    readonly property string highlightTheme: root.settingsModel.highlight_theme.length > 0 ? root.settingsModel.highlight_theme : root.desktopHighlightTheme
    readonly property ThemeColors consoleColors: ThemeColors {
        themeName: root.highlightTheme.length > 0 ? root.highlightTheme : Kirigami.ColorUtils.brightnessForColor(Kirigami.Theme.backgroundColor) === Kirigami.ColorUtils.Dark ? "Breeze Dark" : "Breeze Light"

        onAnsiChanged: root.consoleModel.set_console_palette(root.consoleColors.ansi)
    }
    property string section: "overview"
    property string optionQuery: ""
    property string configuredQuery: ""
    property string homeQuery: ""
    property string optionNamespace: ""
    property bool optionOnlyUser: false
    property bool optionShowTemplated: false
    property string homeNamespace: ""
    property bool homeOnlyUser: false
    property bool homeShowTemplated: false
    property string packageQuery: ""
    property bool setupOpen: false
    readonly property var setupItem: setupOverlay.page
    property bool packageOnlyInstalled: false
    property string functionQuery: ""

    // Recreating a section page would churn its toolbar action list, so each is created once.
    property var sectionPages: ({})

    // A binding would loop (every role notifies through colorsChanged), so the link roles are set when they differ.
    property string appliedLink: ""
    property string appliedVisited: ""

    function docText(markdown: var): string {
        return Helpers.docText(markdown);
    }

    function guideText(markdown: var): string {
        return Helpers.guideText(markdown);
    }

    function openExternal(url: string): void {
        Helpers.openExternal(url);
    }

    // Kirigami pushes Components with a non-visual parent, which logs a "not placed in the graphics scene" warning.
    function createPage(component: Component, properties: var): var {
        const page = component.createObject(root.pageStack, properties || {});
        root.clearPageBackground(page);
        return page;
    }

    function clearPageBackground(page: var): void {
        if (!root.vibrancy || !page)
            return;
        page.background = null;

        const clearToolBar = () => {
            const header = page.globalToolBarItem;
            if (header && header.background)
                root.clearFills(header.background);
        };
        clearToolBar();
        page.globalToolBarItemChanged.connect(clearToolBar);
    }

    function clearFills(item: Item): void {
        if (item.color !== undefined)
            item.color = "transparent";
        for (const child of item.children)
            root.clearFills(child);
    }

    function matchingHighlightTheme(desktop: string): string {
        return Helpers.matchingHighlightTheme(desktop, Repository.themes);
    }

    function showSection(name: string): void {
        if (name === "docs" && root.section === "docs" && root.pageStack.depth > 1)
            return;

        root.openSection(name);

        if (root.section === "docs")
            root.showDoc("introduction.md", "");
    }

    function openSection(name: string): void {
        let component = pages.forSection(name);

        if (!component) {
            name = "overview";
            component = pages.overview;
        }

        root.section = name;

        if (root.pageStack.depth > 1)
            root.pageStack.pop(null);

        let page = root.sectionPages[name];

        if (!page) {
            page = root.createPage(component);
            root.sectionPages[name] = page;
            pages.addHelp(page, name);
        }

        const top = root.pageStack.items[root.pageStack.depth - 1];

        if (top === page)
            root.pageStack.currentIndex = root.pageStack.depth - 1;
        else if (root.pageStack.depth === 0)
            root.pageStack.push(page);
        else
            root.pageStack.replace(page);
    }

    function showOption(name: string, source: var): void {
        const model = source || root.optionModel;

        if (root.section !== "options" && root.section !== "tree" && root.section !== "configured" && root.section !== "home")
            root.showSection("options");

        const top = root.pageStack.items[root.pageStack.depth - 1];

        if (top && top.isOptionDetails) {
            top.options = model;
            top.optionName = name;
            root.pageStack.currentIndex = root.pageStack.depth - 1;
        } else {
            root.pageStack.push(root.createPage(pages.optionDetails, {
                options: model,
                optionName: name
            }));
        }
    }

    function showPackage(attr: string): void {
        Helpers.showDetails(root, "packages", "isPackageDetails", pages.packageDetails, {
            attr: attr
        });
    }

    function showFunction(path: string): void {
        Helpers.showDetails(root, "functions", "isFunctionDetails", pages.functionDetails, {
            path: path
        });
    }

    function showDoc(file: string, anchor: string): void {
        Helpers.showGuide(root, pages.guide, file, anchor);
    }

    function showHelp(section: string): void {
        Helpers.showHelp(root, section);
    }

    function showPreview(mode: string): void {
        const resolved = mode || "";
        const top = root.pageStack.items[root.pageStack.depth - 1];

        if (top && top.isPreview) {
            top.pendingMode = resolved;
            top.refresh();
        } else {
            root.pageStack.push(root.createPage(pages.preview, {
                pendingMode: resolved
            }));
        }
    }

    function toggleFullScreen(): void {
        if (root.fullscreen)
            root.showNormal();
        else
            root.showFullScreen();
    }

    function reviewApply(mode: string): void {
        root.systemModel.review_apply(root.changeModel.plan_json(), mode);
        root.showSection("log");
    }

    function startApply(mode: string): void {
        root.systemModel.apply(root.changeModel.plan_json(), mode);
        root.showSection("log");
    }

    function syncThemeLinks(): void {
        if (root.macos)
            return;
        const dark = Kirigami.ColorUtils.brightnessForColor(Kirigami.Theme.backgroundColor) === Kirigami.ColorUtils.Dark;
        const link = dark ? "#1d99f3" : "#2980b9";
        const visited = dark ? "#9b59b6" : "#7f8c8d";
        if (root.appliedLink !== link) {
            Kirigami.Theme.linkColor = link;
            root.appliedLink = link;
        }
        if (root.appliedVisited !== visited) {
            Kirigami.Theme.visitedLinkColor = visited;
            root.appliedVisited = visited;
        }
    }

    title: root.programOutdated || root.systemModel.host.length === 0 ? qsTr("DrakeFlake") : qsTr("%1 — DrakeFlake").arg(root.systemModel.host)
    width: 1600
    height: 900
    minimumWidth: Kirigami.Units.gridUnit * 22
    minimumHeight: Kirigami.Units.gridUnit * 20
    visible: true
    pageStack.defaultColumnWidth: Kirigami.Units.gridUnit * 26
    pageStack.globalToolBar.style: Kirigami.ApplicationHeaderStyle.ToolBar
    pageStack.globalToolBar.showNavigationButtons: Kirigami.ApplicationHeaderStyle.ShowBackButton
    pageStack.globalToolBar.separatorVisible: !root.macos

    menuBar: AppMenu {
        app: root
        generations: root.systemModel.capabilities.generations
    }
    globalDrawer: NavigationDrawer {
        section: root.section
        windowWidth: root.width
        generations: root.systemModel.capabilities.generations
        changeCount: root.changeModel.count
        running: root.systemModel.running
        macos: root.macos
        unifiedTitlebar: root.unifiedTitlebar
        toolBarHeight: root.pageStack.globalToolBar.preferredHeight - root.menuBar.height - root.titlebarHeight
        vibrancy: root.vibrancy

        onSectionChosen: name => root.showSection(name)
    }
    header: OutdatedBanner {
        outdated: root.programOutdated
        contentX: root.contentItem.x
        windowWidth: root.width
        topClearance: root.titlebarHeight
    }
    footer: StatusBar {
        busy: root.systemModel.busy
        running: root.systemModel.running
        activity: root.systemModel.activity
        consoleShown: root.section === "log"
        clear: root.vibrancy
        reduceMotion: root.reduceMotion
        contentX: root.contentItem.x
        windowWidth: root.width

        onShowConsole: root.showSection("log")
    }

    onVibrancyChanged: {
        for (const name in root.sectionPages)
            root.clearPageBackground(root.sectionPages[name]);
        for (let i = 0; i < root.pageStack.depth; i++)
            root.clearPageBackground(root.pageStack.items[i]);
    }
    onDirtyChanged: {
        if (!root.programOutdated)
            root.systemModel.set_window_dirty(root.dirty);
    }
    // Quitting beats the delayed save below: remember the place at once.
    onClosing: close => {
        windowState.save();
        quitGuard.intercept(close);
    }
    Component.onCompleted: {
        root.syncThemeLinks();
        root.showSection("overview");
        root.setupOpen = root.setupModel.needs_setup;
        root.systemModel.ui_ready();
        if (!root.programOutdated)
            root.systemModel.set_window_dirty(root.dirty);
    }

    Connections {
        function onColorsChanged(): void {
            root.syncThemeLinks();
        }

        target: Kirigami.Theme
    }

    Binding {
        target: root
        property: "color"
        value: root.vibrancy ? (root.macos ? "transparent" : Qt.alpha(root.palette.window, 0.7)) : root.palette.window
    }

    PageComponents {
        id: pages

        app: root
    }

    WindowState {
        id: windowState

        objectName: "windowState"
        app: root
        availableWidth: Screen.desktopAvailableWidth
        availableHeight: Screen.desktopAvailableHeight
    }

    QuitGuard {
        id: quitGuard

        app: root
    }

    Shortcuts {
        app: root
    }

    SetupOverlay {
        id: setupOverlay

        app: root
        shown: root.setupOpen

        onFinished: {
            root.setupModel.finish_setup();
            root.setupOpen = false;
            root.showSection("overview");
        }
    }

    Loader {
        active: root.systemModel.selftest("dir") !== ""
        source: "SelfTest.qml"

        onLoaded: item.app = root
    }
}
