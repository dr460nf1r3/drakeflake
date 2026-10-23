pragma ComponentBehavior: Bound

// qmllint disable import
// (`QtQuick.NativeStyle` only builds on macOS and Windows: Linux qmllint cannot resolve it)
import QtQuick
import QtQuick.Templates as T
import QtQuick.NativeStyle as NativeStyle

// qmllint disable Quick.controls-native-customize
// qmllint disable unresolved-type missing-property unqualified
// (`ScrollBar` comes from `import QtQuick.Controls.macOS`: another macOS-only module)

T.ScrollView {
    id: control

    implicitWidth: Math.max(implicitBackgroundWidth + leftInset + rightInset, implicitContentWidth + leftPadding + rightPadding)
    implicitHeight: Math.max(implicitBackgroundHeight + topInset + bottomInset, implicitContentHeight + topPadding + bottomPadding)
    rightPadding: control.ScrollBar.vertical.implicitWidth + control.padding
    bottomPadding: control.effectiveScrollBarHeight + control.padding

    ScrollBar.vertical: ScrollBar {
        parent: control
        x: control.mirrored ? 0 : control.width - width
        y: 0
        height: control.height - (control.ScrollBar.horizontal.visible ? control.ScrollBar.horizontal.height : 0)
        active: control.ScrollBar.horizontal.active

        NativeStyle.ScrollViewCorner {
            y: parent.height
            control: control
            visible: control.ScrollBar.horizontal.visible
            useNinePatchImage: false
        }
    }
    ScrollBar.horizontal: ScrollBar {
        parent: control
        x: 0
        y: control.height - height
        width: control.width - (control.ScrollBar.vertical.visible ? control.ScrollBar.vertical.width : 0)
        active: control.ScrollBar.vertical.active
    }
}
