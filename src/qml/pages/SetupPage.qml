pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../components"
import "setup"
import "../main/kinds.js" as Kinds

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var system: page.app.systemModel
    readonly property var settingsModel: page.app.settingsModel
    readonly property var setupModel: page.app.setupModel
    property int step: 0
    readonly property var candidates: page.setupModel.flake_candidates
    readonly property bool scanning: page.setupModel.flake_scanning
    readonly property var hostList: page.system.hosts
    readonly property string defaultFlake: page.system.capabilities.default_flake
    property string flakePath: page.candidates.length > 0 ? page.candidates[0] : ""
    readonly property string flakeRoot: page.flakePath.length > 0 ? page.setupModel.flake_root(page.flakePath) : ""
    property string loadedFlake: ""
    readonly property bool loading: page.loadedFlake.length > 0 && !page.system.target_ready && page.system.target_error.length === 0
    property string folder: ""
    readonly property string effectiveFolder: page.setupModel.effective_folder(page.folder, page.flakeRoot)
    property int folderRevision: 0
    readonly property bool folderExists: page.folderRevision >= 0 && page.effectiveFolder.length > 0 && page.setupModel.folder_exists(page.effectiveFolder)
    readonly property bool folderInFlake: page.folderRevision >= 0 && page.flakeRoot.length > 0 && page.setupModel.folder_in_flake(page.effectiveFolder, page.flakeRoot)
    readonly property bool canContinue: page.system.target_ready && page.loadedFlake.length > 0 && page.folderInFlake && page.folderExists
    property string folderError: ""
    property string layoutMode: page.settingsModel.layout_mode
    property bool autoCommit: page.settingsModel.auto_commit
    property bool watchFlake: page.settingsModel.tray_settings.watch_flake === true

    signal finished

    function loadFlake(): void {
        if (page.flakeRoot.length === 0 || page.flakeRoot === page.loadedFlake)
            return;

        page.loadedFlake = page.flakeRoot;

        if (page.system.target_ready && page.setupModel.flake_root(page.system.flake) === page.flakeRoot)
            return;

        page.system.retarget(page.flakeRoot, "", "");
    }

    function createFolder(): void {
        page.folderError = page.setupModel.create_folder(page.effectiveFolder);
        page.folderRevision++;
    }

    function useSystem(): void {
        const output = page.effectiveFolder === page.flakeRoot ? "" : page.effectiveFolder + "/";
        page.folderError = page.settingsModel.set_layout(page.layoutMode, page.settingsModel.layout_rules);

        if (page.folderError.length > 0)
            return;

        page.settingsModel.set_auto_commit(page.autoCommit);
        page.settingsModel.set_tray_report("watch_flake", page.watchFlake);
        page.system.retarget(page.flakeRoot, page.system.host, output);
        page.step = 2;
    }

    title: qsTr("Set up DrakeFlake")

    footer: SetupFooter {
        step: page.step
        canContinue: page.canContinue
        canUseFlake: page.flakeRoot.length > 0 && page.flakeRoot !== page.loadedFlake
        imported: page.setupModel.import_status === Kinds.importImported

        onBack: page.step--
        onUseFlake: page.loadFlake()
        onNext: {
            if (page.step === 0)
                page.step = 1;
            else
                page.useSystem();
        }
        onFinish: page.finished()
    }

    Component.onCompleted: page.setupModel.scan_flakes()

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        Kirigami.Heading {
            Layout.fillWidth: true
            level: 1
            wrapMode: Text.Wrap
            text: page.title
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: Kirigami.Units.largeSpacing

            Repeater {
                model: [qsTr("Requirements"), qsTr("System"), qsTr("Import")]

                delegate: Controls.Label {
                    required property string modelData
                    required property int index

                    text: qsTr("%1. %2").arg(index + 1).arg(modelData)
                    font.bold: index === page.step
                    opacity: index === page.step ? 1 : 0.6
                }
            }

            Item {
                Layout.fillWidth: true
            }
        }

        Kirigami.Separator {
            Layout.fillWidth: true
        }

        StackLayout {
            Layout.fillWidth: true
            currentIndex: page.step

            RequirementsStep {
                scanning: page.scanning
                foundFlake: page.candidates.length > 0
                defaultFlake: page.defaultFlake
            }

            SystemStep {
                scanning: page.scanning
                candidates: page.candidates
                flakePath: page.flakePath
                flakeRoot: page.flakeRoot
                flakeLoaded: page.loadedFlake.length > 0
                loading: page.loading
                targetReady: page.system.target_ready
                targetError: page.system.target_error
                hostList: page.hostList
                defaultFlake: page.defaultFlake
                host: page.system.host
                folder: page.folder
                folderExists: page.folderExists
                folderInFlake: page.folderInFlake
                folderIsFlakeRoot: page.effectiveFolder === page.flakeRoot
                folderError: page.folderError
                layoutMode: page.layoutMode
                autoCommit: page.autoCommit
                watchFlake: page.watchFlake

                onFlakePathChosen: path => page.flakePath = path
                onBrowseFlake: flakePicker.open()
                onHostChosen: host => page.system.retarget(page.flakeRoot, host, "")
                onFolderEdited: text => {
                    page.folder = text;
                    page.folderError = "";
                }
                onBrowseFolder: folderPicker.open()
                onCreateFolder: page.createFolder()
                onLayoutChosen: mode => page.layoutMode = mode
                onAutoCommitChosen: checked => page.autoCommit = checked
                onWatchFlakeChosen: checked => page.watchFlake = checked
            }

            ImportStep {
                setupModel: page.setupModel
                host: page.system.host
                targetReady: page.system.target_ready
                highlightTheme: page.app.highlightTheme
            }
        }
    }

    PortalPicker {
        id: flakePicker

        setupModel: page.setupModel
        purpose: "setup-flake"
        title: qsTr("Choose your system flake")
        startFolder: page.flakePath || page.defaultFlake

        onPicked: path => page.flakePath = path
    }

    PortalPicker {
        id: folderPicker

        setupModel: page.setupModel
        purpose: "setup-folder"
        title: qsTr("Choose the folder for the generated files")
        startFolder: page.effectiveFolder || page.flakeRoot

        onPicked: path => {
            page.folder = path;
            page.folderError = "";
            page.folderRevision++;
        }
    }
}
