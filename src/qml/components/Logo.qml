import QtQuick
import org.kde.kirigami as Kirigami

Kirigami.Icon {
    // Pale ice on a light background disappears: the light theme gets deeper blues.
    source: Qt.resolvedUrl(Kirigami.Theme.backgroundColor.hslLightness > 0.5 ? "../images/drakeflake-logo-light.svg" : "../images/drakeflake-logo.svg")
    fallback: "preferences-system"
}
