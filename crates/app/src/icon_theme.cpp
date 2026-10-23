#include <QtCore/QString>
#include <QtCore/QStringList>
#include <QtGui/QIcon>

extern "C" void drakeflake_use_breeze_icons() {
    QIcon::setThemeName(QStringLiteral("breeze"));
}

extern "C" void drakeflake_use_icon_theme(const char *search_path, const char *name) {
    QStringList paths = QIcon::themeSearchPaths();
    paths.prepend(QString::fromUtf8(search_path));
    QIcon::setThemeSearchPaths(paths);
    QIcon::setThemeName(QString::fromUtf8(name));
}
