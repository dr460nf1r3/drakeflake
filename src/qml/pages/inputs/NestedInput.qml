import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Layouts
import org.kde.kirigami as Kirigami

import "../../components"
import "../../main/helpers.js" as Helpers

ColumnLayout {
    id: child

    required property var modelData
    required property string parentName
    readonly property string relation: {
        if (child.modelData.follows)
            return child.modelData.follows_root ? "follows-yours" : "follows";

        if (child.modelData.same_as)
            return "shared";

        return child.modelData.could_follow ? "duplicate" : "own";
    }

    spacing: 0

    Flow {
        Layout.fillWidth: true
        spacing: Kirigami.Units.smallSpacing

        Controls.Label {
            text: child.modelData.name
            textFormat: Text.PlainText
            font.family: "monospace"
        }

        Tag {
            visible: child.relation === "follows-yours" || child.relation === "shared"
            text: child.relation === "shared" ? qsTr("same as your %1").arg(child.modelData.same_as) : qsTr("follows your %1").arg(child.modelData.follows)
            tone: "positive"
        }

        Tag {
            visible: child.relation === "follows"
            text: qsTr("follows %1").arg(child.modelData.follows || "")
        }

        Tag {
            visible: child.relation === "duplicate"
            text: qsTr("separate copy")
            tone: "warning"
        }

        Tag {
            visible: child.relation === "own"
            text: qsTr("own copy")
        }
    }

    Controls.Label {
        Layout.fillWidth: true
        visible: child.relation === "own" || child.relation === "duplicate"
        opacity: 0.75
        font: Kirigami.Theme.smallFont
        wrapMode: Text.WrapAnywhere
        textFormat: Text.PlainText
        text: {
            const rev = Helpers.shortRev(child.modelData.rev || "");
            const date = Helpers.dateWithAge(child.modelData.last_modified);
            const where = rev.length > 0 ? qsTr("%1 at %2").arg(child.modelData.source || "").arg(rev) : (child.modelData.source || "");
            return date.length > 0 ? qsTr("%1 · %2").arg(where).arg(date) : where;
        }
    }

    Controls.Label {
        Layout.fillWidth: true
        visible: child.relation === "duplicate"
        color: Kirigami.Theme.neutralTextColor
        font: Kirigami.Theme.smallFont
        wrapMode: Text.Wrap
        textFormat: Text.PlainText
        text: qsTr("Your input “%1” comes from the same repository. Following it avoids a second copy: inputs.%2.inputs.%3.follows = \"%1\";").arg(child.modelData.could_follow || "").arg(child.parentName).arg(child.modelData.name)
    }
}
