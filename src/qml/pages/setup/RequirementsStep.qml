import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

ColumnLayout {
    id: step

    required property bool scanning
    required property bool foundFlake
    required property string defaultFlake

    spacing: Kirigami.Units.largeSpacing

    Controls.Label {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        textFormat: Text.MarkdownText
        text: qsTr("DrakeFlake edits your NixOS or nix-darwin configuration by writing generated `.nix` files into your system flake. Before you start, you need:")
    }

    Controls.Label {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        textFormat: Text.MarkdownText
        text: qsTr("- **A system flake**: a folder with a `flake.nix` that defines `nixosConfigurations` or, for nix-darwin, `darwinConfigurations`.\n- **A host already set up in it**: the configuration of this machine, e.g. `nixosConfigurations.<hostname>`.\n- **For Home Manager settings**: Home Manager imported as a system module (`home-manager.nixosModules.home-manager` or `home-manager.darwinModules.home-manager`) with your user set up in `home-manager.users.<name>`.")
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: true
        type: Kirigami.MessageType.Information
        text: qsTr("Your changes only take effect once your host configuration imports the generated file. The last step shows the line to add and checks that it is in place.")
    }

    RowLayout {
        visible: step.scanning

        Controls.BusyIndicator {
            Layout.preferredWidth: Kirigami.Units.iconSizes.sizeForLabels
            Layout.preferredHeight: Kirigami.Units.iconSizes.sizeForLabels
            running: step.scanning
        }

        Controls.Label {
            text: qsTr("Looking for system flakes…")
        }
    }

    Kirigami.InlineMessage {
        Layout.fillWidth: true
        visible: !step.scanning && !step.foundFlake
        type: Kirigami.MessageType.Warning
        text: step.defaultFlake === "/etc/nix-darwin" ? qsTr("No system flake was found in /etc/nix-darwin or your home folder. You can choose its folder in the next step, or create one first, for example with “nix flake init -t nix-darwin”.") : qsTr("No system flake was found in /etc/nixos or your home folder. You can choose its folder in the next step, or create one first, for example with “sudo nixos-generate-config --flake”.")
    }
}
