import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami
import "../../main/kinds.js" as Kinds

ColumnLayout {
    id: picker

    required property string scopeHost
    required property string currentHost
    required property string hostState

    signal picked(string host)

    spacing: Kirigami.Units.smallSpacing

    Controls.ComboBox {
        id: scopeBox

        objectName: "scopeBox"
        Layout.minimumWidth: Kirigami.Units.gridUnit * 12
        textRole: "text"
        model: [
            {
                text: qsTr("All hosts that import the file")
            },
            {
                text: picker.scopeHost.length > 0 ? qsTr("Only %1").arg(picker.scopeHost) : picker.currentHost.length > 0 ? qsTr("Only this host (%1)").arg(picker.currentHost) : qsTr("Only this host")
            }
        ]
        currentIndex: picker.scopeHost.length > 0 ? 1 : 0
        enabled: picker.scopeHost.length > 0 || picker.currentHost.length > 0

        onActivated: index => picker.picked(index === 1 ? (picker.scopeHost.length > 0 ? picker.scopeHost : picker.currentHost) : "")
    }

    Controls.Label {
        Layout.fillWidth: true
        wrapMode: Text.Wrap
        textFormat: Text.PlainText
        font: Kirigami.Theme.smallFont
        opacity: 0.75
        text: {
            if (picker.scopeHost.length > 0)
                return qsTr("Written under lib.mkIf (config.networking.hostName == \"%1\"): other hosts importing the file ignore it.").arg(picker.scopeHost);
            if (picker.currentHost.length > 0)
                return qsTr("Every host that imports the generated file gets this setting.");
            if (picker.hostState === Kinds.valueLoading)
                return qsTr("Evaluating networking.hostName to offer \"only this host\"…");
            return qsTr("\"Only this host\" needs the evaluated networking.hostName of this system, which is not known.");
        }
    }
}
