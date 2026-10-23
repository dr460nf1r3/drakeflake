.pragma library
.import QtQml as QtQml

// NixOS docs use MyST roles ({option}`x`) that Qt's Markdown does not know.
function docText(markdown) {
    return escapeRawHtml(guideText(markdown));
}

// The app's own guide is trusted, so its inline HTML (<kbd>, <em>) is rendered. Flake text goes through docText.
function guideText(markdown) {
    if (!markdown)
        return "";

    // Qt would fetch a remote image of any flake input's documentation as soon as it is shown.
    return markdown.replace(/!\[([^\]]*)\](\([^)]*\)|\[[^\]]*\])?/g, "$1").replace(/\{(option|var|env|command|file|manpage|code)\}`([^`]*)`/g, "`$2`").replace(/:::\s*\{\.(\w+)\}/g, "**$1:**").replace(/^:::\s*$/gm, "");
}

// Qt's Markdown renders inline HTML: <img> loads its source and <a> links any
// scheme, and so do <https://...> autolinks and entities, which only yield text.
function escapeRawHtml(markdown) {
    let fence = "";
    return markdown.split("\n").map(line => {
        const marker = /^ {0,3}(`{3,}|~{3,})/.exec(line);
        if (fence) {
            if (marker && marker[1][0] === fence[0] && marker[1].length >= fence.length)
                fence = "";
            return line;
        }
        if (marker) {
            fence = marker[1];
            return line;
        }
        return line.split(/(`+[^`]+?`+)/).map((part, index) => index % 2 === 1 ? part : escapeOutsideCode(part)).join("");
    }).join("\n");
}

function escapeOutsideCode(text) {
    return text.split(/(<https?:\/\/[^\s<>]+>)/i).map((part, index) => index % 2 === 1 ? part : part.replace(/&(?!(?:#\d+|#x[0-9a-f]+|[a-z][a-z0-9]*);)/gi, "&amp;").replace(/</g, "&lt;")).join("");
}

// StyledText hands back an href with the escaping of `link()` still in it.
function unescapeHtml(text) {
    return String(text).replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&quot;/g, "\"").replace(/&amp;/g, "&");
}

function openExternal(url) {
    const target = unescapeHtml(url);

    // Links come from flake inputs: file:// or a custom handler could open local files or programs.
    if (/^https?:\/\//i.test(target))
        Qt.openUrlExternally(target);
    else
        console.warn("not opening a link that is not http(s):", target);
}

function escapeHtml(text) {
    return String(text).replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");
}

// Tooltips and InlineMessages guess rich text from the first tag, so the span forces it and flake text stays escaped.
function richText(text) {
    return "<span>" + escapeHtml(text) + "</span>";
}

// The href is escaped too: a quote in a URL would end the attribute.
function link(url, label) {
    return "<a href=\"" + escapeHtml(url) + "\">" + escapeHtml(label) + "</a>";
}

const SECONDS_PER_DAY = 86400;
const BYTES_PER_GIB = 1073741824;

function formatDate(seconds) {
    return seconds ? new Date(seconds * 1000).toLocaleString(Qt.locale(), QtQml.Locale.ShortFormat) : "";
}

function ageInDays(seconds) {
    return Math.floor((Date.now() / 1000 - seconds) / SECONDS_PER_DAY);
}

function dateWithAge(seconds) {
    if (!seconds)
        return "";

    const days = ageInDays(seconds);
    const date = new Date(seconds * 1000).toLocaleDateString(Qt.locale(), QtQml.Locale.ShortFormat);

    if (days < 1)
        return qsTr("%1 (today)").arg(date);

    return days === 1 ? qsTr("%1 (yesterday)").arg(date) : qsTr("%1 (%2 days ago)").arg(date).arg(days);
}

const HELP_TOPICS = {
    overview: "overview.md",
    options: "options.md",
    tree: "options.md#browse-all-options-as-a-tree",
    configured: "options.md#see-everything-your-host-sets",
    home: "home-manager.md",
    packages: "packages.md",
    functions: "functions.md",
    generations: "maintenance.md#generations",
    actions: "maintenance.md#actions",
    cleanup: "maintenance.md#cleanup",
    inputs: "inputs.md",
    impermanence: "impermanence.md",
    history: "applying.md#history-and-restoring",
    changes: "applying.md",
    log: "applying.md#the-console",
    settings: "settings.md",
    about: "introduction.md"
};

function helpTopic(section) {
    return HELP_TOPICS[section] || "";
}

function sectionOffered(section, macos) {
    return section !== "impermanence" || !macos;
}

function guideLink(link, currentFile) {
    const match = /^([A-Za-z0-9_-]+\.md)?(?:#([A-Za-z0-9_-]+))?$/.exec(link);

    if (!match || (!match[1] && !match[2]))
        return null;

    return {
        file: match[1] || currentFile,
        anchor: match[2] || ""
    };
}

function showDetails(app, section, flag, component, properties) {
    if (app.section !== section)
        app.showSection(section);

    const top = app.pageStack.items[app.pageStack.depth - 1];

    if (top && top[flag]) {
        for (const key in properties)
            top[key] = properties[key];
        app.pageStack.currentIndex = app.pageStack.depth - 1;
    } else {
        app.pageStack.push(app.createPage(component, properties));
    }
}

function showGuide(app, component, file, anchor) {
    if (app.section !== "docs")
        app.openSection("docs");

    app.sectionPages.docs.currentFile = file;

    if (!app.sectionPages.guide)
        app.sectionPages.guide = app.createPage(component);

    // The blocks are built before the push, so the fade shows the whole page.
    app.sectionPages.guide.open(file, anchor);

    if (app.pageStack.items[app.pageStack.depth - 1] !== app.sectionPages.guide)
        app.pageStack.push(app.sectionPages.guide);

    app.pageStack.currentIndex = app.pageStack.depth - 1;
}

function showHelp(app, section) {
    const link = guideLink(helpTopic(section) || "introduction.md", "");

    app.showDoc(link.file, link.anchor);
}

function goBack(pageStack) {
    if (pageStack.depth > 1)
        pageStack.pop();
}

function focusSearch(page) {
    if (page && typeof page.focusSearch === "function")
        page.focusSearch();
}

// Review comes first, so an accidental keypress cannot rebuild the machine.
function applyPrimary(app) {
    app.showSection("changes");
    const page = app.sectionPages.changes;

    if (page && typeof page.applyPrimary === "function")
        page.applyPrimary();
}

function shortRev(rev) {
    return rev ? rev.slice(0, 12) : "";
}

function count(n) {
    return n.toLocaleString(Qt.locale(), "f", 0);
}

function gib(bytes) {
    return qsTr("%1 GiB").arg(((bytes || 0) / BYTES_PER_GIB).toLocaleString(Qt.locale(), "f", 1));
}

// The longest match wins, so "Catppuccin-Mocha-Mauve" finds "Catppuccin Mocha" and no shorter name in it.
function matchingHighlightTheme(desktop, themes) {
    const key = desktop.toLowerCase().replace(/[^a-z0-9]/g, "");
    let best = "";
    let bestLength = 0;

    for (const theme of themes) {
        const name = theme.name.toLowerCase().replace(/[^a-z0-9]/g, "");

        if (name.length > bestLength && key.indexOf(name) >= 0) {
            best = theme.name;
            bestLength = name.length;
        }
    }

    return best;
}
