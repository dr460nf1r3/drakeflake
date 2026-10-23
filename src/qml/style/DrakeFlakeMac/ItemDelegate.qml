// qmllint disable import
// (`QtQuick.NativeStyle` only builds on macOS and Windows: Linux qmllint cannot resolve it)
import QtQuick
import QtQuick.Templates as T
import QtQuick.NativeStyle as NativeStyle
import org.kde.kirigami as Kirigami

// qmllint disable Quick.controls-native-customize

T.ItemDelegate {
    id: control

    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset, implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset, implicitContentHeight + topPadding + bottomPadding, implicitIndicatorHeight + topPadding + bottomPadding)
    padding: 6
    leftPadding: "tAction" in control ? control.padding + Kirigami.Units.largeSpacing + (Kirigami.Units.iconSizes.medium - Kirigami.Units.iconSizes.smallMedium) / 2 : control.padding
    spacing: 6
    icon.width: 16
    icon.height: 16

    contentItem: NativeStyle.DefaultItemDelegateIconLabel {
    }
    background: Rectangle {
        implicitWidth: 100
        implicitHeight: 20
        radius: Kirigami.Units.cornerRadius
        color: {
            if (control.highlighted || control.checked)
                return Kirigami.Theme.highlightColor;
            if (control.down)
                return Qt.alpha(Kirigami.Theme.textColor, 0.12);
            return control.hovered ? Qt.alpha(Kirigami.Theme.textColor, 0.06) : "transparent";
        }
    }
}
