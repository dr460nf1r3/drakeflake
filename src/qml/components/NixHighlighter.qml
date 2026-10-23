import QtQuick
import org.kde.syntaxhighlighting

SyntaxHighlighter {
    id: highlighter

    property string themeName: ""
    property bool dark: false
    readonly property ThemeColors colors: ThemeColors {
        themeName: highlighter.themeName
    }
    readonly property color backgroundColor: highlighter.colors.background
    readonly property color textColor: highlighter.colors.text
    readonly property bool hasColors: highlighter.colors.loaded

    definition: "Nix"
    theme: {
        if (highlighter.themeName.length > 0) {
            const chosen = Repository.theme(highlighter.themeName);

            if (chosen.name === highlighter.themeName)
                return chosen;
        }

        return Repository.defaultTheme(highlighter.dark ? Repository.DarkTheme : Repository.LightTheme);
    }

    Component.onCompleted: {
        if (Repository.definitionForName("Nix").name !== "Nix")
            console.warn("Nix syntax definition not found; code is shown without highlighting");
    }
}
