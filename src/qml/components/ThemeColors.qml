import QtQuick

// Theme colours read from compiled files because names must map to lower case with dashes.
QtObject {
    id: colors

    property string themeName: ""
    property color background: "transparent"
    property color text: "transparent"
    property var ansi: []
    readonly property bool loaded: colors.background.a > 0

    function load(): void {
        const name = colors.themeName;

        colors.background = "transparent";
        colors.text = "transparent";
        colors.ansi = [];

        if (name.length === 0)
            return;

        const request = new XMLHttpRequest();

        request.onreadystatechange = () => {
            if (request.readyState !== XMLHttpRequest.DONE || name !== colors.themeName)
                return;

            try {
                const theme = JSON.parse(request.responseText);
                const styles = theme["text-styles"] || {};
                const editor = theme["editor-colors"] || {};

                // Array, not rest parameters: qmlformat breaks those.
                const style = names => {
                    for (const key of names) {
                        if (styles[key] && styles[key]["text-color"])
                            return styles[key]["text-color"];
                    }
                    return "";
                };
                const normal = style(["Normal"]);

                if (!editor.BackgroundColor || !normal)
                    return;

                colors.background = editor.BackgroundColor;
                colors.text = normal;

                colors.ansi = [style(["Comment"]) || normal, style(["Error", "Alert"]) || normal, style(["String"]) || normal, style(["Warning", "DecVal"]) || normal, style(["Function", "Keyword"]) || normal, style(["ControlFlow", "Keyword"]) || normal, style(["DataType", "Import"]) || normal, normal];
            } catch (error) {}
        };
        request.open("GET", "qrc:/org.kde.syntax-highlighting/themes/" + name.toLowerCase().replace(/\s+/g, "-") + ".theme");
        request.send();
    }

    onThemeNameChanged: colors.load()
    Component.onCompleted: colors.load()
}
