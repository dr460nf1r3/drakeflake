pragma ComponentBehavior: Bound

import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../components"
import "../main/helpers.js" as Helpers
import "actions/recommendations.js" as Recommendations
import "../main/kinds.js" as Kinds

Kirigami.ScrollablePage {
    id: page

    required property var app
    readonly property var system: page.app.systemModel
    readonly property var generationsModel: page.app.generationsModel
    readonly property var imagesModel: page.app.imagesModel
    readonly property bool canImage: page.app.systemModel.capabilities.images
    readonly property bool darwin: page.app.systemModel.kind === "darwin"
    readonly property var inputsModel: page.app.inputsModel
    readonly property var formats: page.imagesModel.formats
    readonly property var labels: ({
            "iso": qsTr("Live ISO"),
            "iso-installer": qsTr("Installer ISO"),
            "qemu": qsTr("QEMU disk"),
            "qemu-efi": qsTr("QEMU disk (UEFI)"),
            "raw": qsTr("Raw disk"),
            "raw-efi": qsTr("Raw disk (UEFI)"),
            "sd-card": qsTr("SD card"),
            "virtualbox": qsTr("VirtualBox"),
            "vmware": qsTr("VMware"),
            "hyperv": qsTr("Hyper-V"),
            "proxmox": qsTr("Proxmox VM"),
            "proxmox-lxc": qsTr("Proxmox container"),
            "lxc": qsTr("LXC container"),
            "kexec": qsTr("kexec bundle"),
            "oci": qsTr("Oracle Cloud"),
            "amazon": qsTr("Amazon EC2"),
            "azure": qsTr("Azure"),
            "google-compute": qsTr("Google Compute Engine"),
            "digital-ocean": qsTr("DigitalOcean"),
            "linode": qsTr("Linode"),
            "openstack": qsTr("OpenStack"),
            "vagrant-virtualbox": qsTr("Vagrant (VirtualBox)")
        })
    property string format: "iso"
    property string imageHost: ""
    readonly property string shownHost: page.imageHost.length > 0 ? page.imageHost : page.system.host
    readonly property var hostList: page.system.hosts.filter(host => host.kind === "nixos")
    readonly property var checkedOptions: ["documentation.nixos.enable", "programs.nh.enable", "nix.settings.auto-optimise-store", "nix.optimise.automatic", "nix.gc.automatic", "programs.nh.clean.enable"]
    readonly property bool startable: ["iso", "iso-installer", "qemu", "qemu-efi", "raw", "raw-efi"].indexOf(page.imagesModel.format) >= 0

    // Replaced only when they change: rebuilding mid-use destroys the buttons and Breeze logs "Value is null".
    property var shownRecommendations: []
    readonly property var recommendations: {
        page.app.optionModel.revision;
        page.app.changeModel.revision;

        if (!page.app.optionModel.values_ready)
            return [];

        const generations = page.app.systemModel.capabilities.generations ? page.generationsModel.generations : [];
        return Recommendations.list(page.app.optionModel, page.app.changeModel, page.darwin, generations, page.inputsModel.inputs);
    }

    function requestValues(): void {
        if (!page.app.optionModel.values_ready)
            return;

        const names = page.checkedOptions.slice();

        if (page.darwin)
            names.push("system.primaryUser", "programs.nh.clean.user");

        for (const name of names) {
            if (page.app.optionModel.details(name).valueState === Kinds.valueUnknown)
                page.app.optionModel.request_value(name);
        }
    }

    function follow(item: var): void {
        if (item.option) {
            if (item.also) {
                for (const extra of item.also) {
                    if (extra.option && extra.input && extra.type)
                        page.app.changeModel.stage_option(extra.option, extra.type, extra.input);
                }
            }

            const details = page.app.optionModel.details(item.option);
            const error = page.app.changeModel.stage_option(item.option, details.type, item.input);
            if (error.length === 0)
                page.app.showSection("changes");
        } else if (item.search) {
            page.app.optionQuery = item.search;
            page.app.showOption(item.search);
        } else if (item.section) {
            page.app.showSection(item.section);
        }
    }

    title: qsTr("Actions")

    onRecommendationsChanged: {
        if (JSON.stringify(page.recommendations) !== JSON.stringify(page.shownRecommendations))
            page.shownRecommendations = page.recommendations;
    }
    Component.onCompleted: {
        page.shownRecommendations = page.recommendations;
        page.generationsModel.load_generations();
        if (page.canImage && page.formats.length === 0)
            page.imagesModel.load_formats(page.imageHost);
        page.requestValues();

        if (page.inputsModel.inputs.length === 0)
            page.inputsModel.load(false);
    }

    Connections {
        function onState_changed(): void {
            page.requestValues();
        }

        target: page.app.optionModel
    }

    ColumnLayout {
        spacing: Kirigami.Units.largeSpacing

        SectionHeading {
            Layout.topMargin: 0
            visible: page.canImage
            text: qsTr("Create images")
        }

        Controls.Label {
            Layout.fillWidth: true
            visible: page.canImage
            wrapMode: Text.Wrap
            opacity: 0.75
            text: qsTr("Build this system (%1) as an image: a live or installer ISO, a VM disk, an SD card image, a container or a cloud image. Building can take a while. Progress shows in the Console.").arg(page.shownHost)
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: page.canImage && page.imagesModel.error.length > 0
            type: Kirigami.MessageType.Warning
            text: Helpers.richText(page.imagesModel.error)
        }

        RowLayout {
            visible: page.canImage
            spacing: Kirigami.Units.smallSpacing

            FilterPicker {
                options: page.hostList.map(host => ({
                            value: host.name,
                            label: host.name === page.system.host ? qsTr("%1 (this system)").arg(host.name) : host.name
                        }))
                currentValue: page.shownHost
                filterPlaceholder: qsTr("Filter systems…")
                visible: page.hostList.length > 1

                onPicked: value => {
                    page.imageHost = value === page.system.host ? "" : value;
                    page.imagesModel.load_formats(page.imageHost);
                }
            }

            FilterPicker {
                options: page.formats.map(name => ({
                            value: name,
                            label: page.labels[name] ? qsTr("%1 (%2)").arg(page.labels[name]).arg(name) : name
                        }))
                currentValue: page.format
                filterPlaceholder: qsTr("Filter formats…")
                enabled: page.formats.length > 0

                onPicked: value => page.format = value
            }

            Controls.Button {
                text: qsTr("Build image")
                icon.name: "run-build"
                icon.width: Kirigami.Units.iconSizes.sizeForLabels
                icon.height: Kirigami.Units.iconSizes.sizeForLabels
                highlighted: true
                enabled: page.formats.indexOf(page.format) >= 0 && !page.system.running

                onClicked: {
                    page.system.build_image(page.imageHost, page.format);
                    page.app.showSection("log");
                }
            }

            Controls.BusyIndicator {
                Layout.preferredWidth: Kirigami.Units.iconSizes.smallMedium
                Layout.preferredHeight: Kirigami.Units.iconSizes.smallMedium
                visible: page.imagesModel.loading
                running: visible
            }
        }

        Kirigami.InlineMessage {
            Layout.fillWidth: true
            visible: page.canImage && page.imagesModel.file.length > 0
            type: Kirigami.MessageType.Positive
            text: Helpers.richText(qsTr("%1 image ready: %2").arg(page.labels[page.imagesModel.format] || page.imagesModel.format).arg(page.imagesModel.file))

            actions: [
                Kirigami.Action {
                    text: qsTr("Save as…")
                    icon.name: "document-save-as"
                    enabled: !page.imagesModel.saving

                    onTriggered: saveDialog.open()
                },
                Kirigami.Action {
                    text: qsTr("Show in folder")
                    icon.name: "folder-open"

                    onTriggered: page.imagesModel.show()
                },
                Kirigami.Action {
                    text: qsTr("Start in VM")
                    icon.name: "media-playback-start"
                    visible: page.startable
                    tooltip: qsTr("Boot it in QEMU. The image itself is not changed")

                    onTriggered: page.imagesModel.start()
                }
            ]
        }

        SectionHeading {
            Layout.topMargin: page.canImage ? Kirigami.Units.largeSpacing : 0
            text: qsTr("Recommendations")
        }

        Controls.Label {
            Layout.fillWidth: true
            visible: page.shownRecommendations.length === 0
            opacity: 0.75
            wrapMode: Text.Wrap
            textFormat: Text.PlainText
            text: page.app.optionModel.values_ready ? qsTr("Nothing to recommend: your configuration already does all of this.") : qsTr("Recommendations appear once the current values are evaluated.")
        }

        Repeater {
            model: page.shownRecommendations

            delegate: Rectangle {
                id: suggestion

                required property var modelData

                Layout.fillWidth: true
                implicitHeight: messageRow.implicitHeight + Kirigami.Units.largeSpacing * 2
                radius: Kirigami.Units.cornerRadius * 2
                color: Kirigami.ColorUtils.linearInterpolation(Kirigami.Theme.backgroundColor, Kirigami.Theme.textColor, 0.06)

                RowLayout {
                    id: messageRow

                    anchors.fill: parent
                    anchors.margins: Kirigami.Units.largeSpacing
                    spacing: Kirigami.Units.largeSpacing

                    Kirigami.Icon {
                        source: "help-hint"
                        isMask: true
                        color: Kirigami.Theme.highlightColor
                        Layout.preferredWidth: Kirigami.Units.iconSizes.smallMedium
                        Layout.preferredHeight: Kirigami.Units.iconSizes.smallMedium
                        Layout.alignment: Qt.AlignTop
                    }

                    ColumnLayout {
                        Layout.fillWidth: true
                        spacing: 0

                        Controls.Label {
                            Layout.fillWidth: true
                            wrapMode: Text.Wrap
                            textFormat: Text.PlainText
                            font.bold: true
                            text: suggestion.modelData.title
                        }

                        Controls.Label {
                            Layout.fillWidth: true
                            wrapMode: Text.Wrap
                            textFormat: Text.PlainText
                            opacity: 0.85
                            text: suggestion.modelData.text
                        }
                    }

                    IconButton {
                        Layout.alignment: Qt.AlignVCenter
                        text: suggestion.modelData.option ? qsTr("Stage this change") : suggestion.modelData.search ? qsTr("Show them") : qsTr("Open")
                        icon.name: suggestion.modelData.option ? "list-add" : "go-next"

                        onClicked: page.follow(suggestion.modelData)
                    }
                }
            }
        }
    }

    PortalPicker {
        id: saveDialog

        setupModel: page.app.setupModel
        purpose: "actions-image"
        folder: false
        title: qsTr("Save the image")
        suggestedName: page.imagesModel.file.split("/").pop() || "image"
        nameFilters: [qsTr("All files (*)")]
        defaultSuffix: ""

        onPicked: path => page.imagesModel.save(path)
    }
}
