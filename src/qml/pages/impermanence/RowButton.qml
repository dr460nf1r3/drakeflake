import QtQuick.Controls as Controls
import org.kde.kirigami as Kirigami

Controls.Button {
    id: button

    property string hint: ""

    icon.width: Kirigami.Units.iconSizes.sizeForLabels
    icon.height: Kirigami.Units.iconSizes.sizeForLabels
    Controls.ToolTip.visible: button.hovered && button.hint.length > 0
    Controls.ToolTip.text: button.hint
}
