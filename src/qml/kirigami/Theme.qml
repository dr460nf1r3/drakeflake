import QtQuick
import org.kde.kirigami.platform as Platform
import DrakeFlake.Native

Platform.BasicThemeDefinition {
    textColor: SystemTheme.label
    disabledTextColor: SystemTheme.disabledControlText
    highlightColor: SystemTheme.selectedContentBackground
    highlightedTextColor: SystemTheme.alternateSelectedControlText
    backgroundColor: SystemTheme.windowBackground
    alternateBackgroundColor: SystemTheme.alternatingContentBackground1
    focusColor: SystemTheme.keyboardFocusIndicator
    hoverColor: SystemTheme.controlAccent
    activeTextColor: SystemTheme.controlAccent
    activeBackgroundColor: Qt.alpha(SystemTheme.controlAccent, 0.2)
    linkColor: SystemTheme.controlAccent
    linkBackgroundColor: Qt.alpha(SystemTheme.controlAccent, 0.2)
    visitedLinkColor: SystemTheme.controlAccent
    visitedLinkBackgroundColor: Qt.alpha(SystemTheme.controlAccent, 0.2)
    negativeTextColor: SystemTheme.systemRed
    negativeBackgroundColor: Qt.alpha(SystemTheme.systemRed, 0.2)
    neutralTextColor: SystemTheme.systemOrange
    neutralBackgroundColor: Qt.alpha(SystemTheme.systemOrange, 0.2)
    positiveTextColor: SystemTheme.systemGreen
    positiveBackgroundColor: Qt.alpha(SystemTheme.systemGreen, 0.2)
    buttonTextColor: SystemTheme.controlText
    buttonBackgroundColor: SystemTheme.control
    buttonAlternateBackgroundColor: SystemTheme.alternatingContentBackground1
    buttonHoverColor: SystemTheme.controlAccent
    buttonFocusColor: SystemTheme.keyboardFocusIndicator
    viewTextColor: SystemTheme.text
    viewBackgroundColor: SystemTheme.controlBackground
    viewAlternateBackgroundColor: SystemTheme.alternatingContentBackground1
    viewHoverColor: SystemTheme.controlAccent
    viewFocusColor: SystemTheme.keyboardFocusIndicator
    selectionTextColor: SystemTheme.alternateSelectedControlText
    selectionBackgroundColor: SystemTheme.selectedContentBackground
    selectionAlternateBackgroundColor: SystemTheme.unemphasizedSelectedContentBackground
    selectionHoverColor: SystemTheme.controlAccent
    selectionFocusColor: SystemTheme.keyboardFocusIndicator
    tooltipTextColor: SystemTheme.label
    tooltipBackgroundColor: SystemTheme.windowBackground
    tooltipAlternateBackgroundColor: SystemTheme.alternatingContentBackground1
    tooltipHoverColor: SystemTheme.controlAccent
    tooltipFocusColor: SystemTheme.keyboardFocusIndicator
    headerTextColor: SystemTheme.headerText
    headerBackgroundColor: SystemTheme.windowBackground
    headerAlternateBackgroundColor: SystemTheme.alternatingContentBackground1
    headerHoverColor: SystemTheme.controlAccent
    headerFocusColor: SystemTheme.keyboardFocusIndicator
    defaultFont: SystemTheme.defaultFont
    smallFont: SystemTheme.smallFont
}
