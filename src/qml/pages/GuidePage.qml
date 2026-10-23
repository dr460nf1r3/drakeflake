pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../main/helpers.js" as Helpers
import "docs"

Kirigami.ScrollablePage {
    id: page

    required property var app
    property string file: ""
    property string anchor: ""
    property real snappedTop: -1
    property bool anchorOwed: true
    property bool anchoring: false
    readonly property bool isGuide: true
    readonly property bool settling: settleTimer.running || scrollTimer.running
    property var guide: ({
            title: "",
            blocks: [],
            error: ""
        })
    readonly property int scrollDelay: 50
    readonly property int settleTime: 1000

    function open(file: string, anchor: string): void {
        if (file === page.file && page.guide.blocks.length > 0) {
            page.anchor = anchor;
            page.anchorOwed = true;
            page.snappedTop = -1;
            scrollTimer.restart();
            return;
        }

        page.file = file;
        page.anchor = anchor;
        page.load();
    }

    function load(): void {
        page.guide = page.app.docsModel.page(page.file);

        if (page.anchor !== "" && !page.guide.blocks.some(block => block.kind === "heading" && block.anchor === page.anchor))
            console.warn("the guide page " + page.file + " has no heading #" + page.anchor);

        page.anchorOwed = true;
        page.snappedTop = -1;

        if (page.anchor === "")
            page.snapTo(0);

        // Text wraps over several layout passes after this returns.
        settleTimer.restart();
        scrollTimer.restart();
    }

    function snapTo(top: real): void {
        page.anchoring = true;
        page.flickable.contentY = Math.max(0, Math.min(top, page.flickable.contentHeight - page.flickable.height));
        page.anchoring = false;
    }

    function scrollToAnchor(): void {
        const top = page.anchorTop();

        if (top < 0)
            return;

        if (page.snappedTop >= 0 && Math.abs(top - page.snappedTop) < 1) {
            page.anchorOwed = false;
            return;
        }

        page.snappedTop = top;
        page.snapTo(top);
    }

    function anchorTop(): real {
        if (!page.anchorOwed)
            return -1;

        const target = page.anchor === "" ? null : blocks.headingItem(page.anchor);

        if (page.anchor !== "" && !target)
            return -1;

        const top = target ? target.mapToItem(page.flickable.contentItem, 0, 0).y - Kirigami.Units.largeSpacing : 0;
        return Math.max(0, Math.min(top, page.flickable.contentHeight - page.flickable.height));
    }

    function follow(link: string): void {
        const target = Helpers.guideLink(link, page.file);

        if (target)
            page.app.showDoc(target.file, target.anchor);
        else
            page.app.openExternal(link);
    }

    title: page.guide.title || qsTr("Documentation")
    verticalScrollBarPolicy: Controls.ScrollBar.AlwaysOn

    header: Kirigami.InlineMessage {
        objectName: "guideError"
        visible: page.guide.error !== ""
        position: Kirigami.InlineMessage.Position.Header
        type: Kirigami.MessageType.Error
        text: Helpers.richText(qsTr("This page of the documentation could not be read: %1").arg(page.guide.error))
    }

    // The cached page is created without a file: showGuide opens it after.
    Component.onCompleted: {
        if (page.file !== "")
            page.load();
    }

    Timer {
        id: scrollTimer

        interval: page.scrollDelay

        onTriggered: page.scrollToAnchor()
    }

    Timer {
        id: settleTimer

        interval: page.settleTime
    }

    Connections {
        function onContentHeightChanged(): void {
            if (!settleTimer.running)
                return;

            const top = page.anchorTop();

            if (top >= 0 && (page.snappedTop < 0 || Math.abs(top - page.snappedTop) >= 1))
                scrollTimer.restart();
        }

        function onContentYChanged(): void {
            if (!page.anchoring)
                page.anchorOwed = false;
        }

        target: page.flickable
    }

    ColumnLayout {
        GuideBlocks {
            id: blocks

            objectName: "guideBlocks"
            Layout.fillWidth: true
            Layout.margins: Kirigami.Units.largeSpacing
            app: page.app
            blocks: page.guide.blocks

            onLinkActivated: link => page.follow(link)
        }
    }
}
