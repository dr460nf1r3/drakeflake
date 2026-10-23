// The blur uses Qt's private Cocoa window interface (Qt 6.11).
#import <AppKit/AppKit.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>
#include <objc/message.h>

#include <QtCore/QEvent>
#include <QtCore/QMetaObject>
#include <QtCore/QObject>
#include <QtCore/QPoint>
#include <QtCore/QRect>
#include <QtGui/QColor>
#include <QtGui/QFont>
#include <QtGui/QFontInfo>
#include <QtGui/QFontDatabase>
#include <QtGui/QGuiApplication>
#include <QtGui/QStyleHints>
#include <QtGui/QSurfaceFormat>
#include <QtGui/QWindow>
#include <QtGui/qpa/qplatformwindow_p.h>
#include <QtQml/QQmlPropertyMap>
#include <QtQml/qqml.h>

namespace {
// AppKit's values (NSVisualEffectView.h). The private Qt header only declares the enum types.
constexpr NSInteger material_sidebar = 7;
constexpr NSInteger blending_behind_window = 0;
constexpr NSInteger state_follows_window = 0;
constexpr quintptr whole_window = 1;

// Lists that change here change in src/qml/DrakeFlake/Native/SystemTheme.qml too.
const char *const color_names[] = {
    "labelColor", "secondaryLabelColor", "tertiaryLabelColor", "quaternaryLabelColor",
    "textColor", "placeholderTextColor", "selectedTextColor", "textBackgroundColor",
    "selectedTextBackgroundColor", "keyboardFocusIndicatorColor",
    "unemphasizedSelectedTextColor", "unemphasizedSelectedTextBackgroundColor",
    "linkColor", "separatorColor", "selectedContentBackgroundColor",
    "unemphasizedSelectedContentBackgroundColor", "selectedMenuItemTextColor", "gridColor",
    "headerTextColor", "windowBackgroundColor", "underPageBackgroundColor",
    "controlBackgroundColor", "controlColor", "controlTextColor", "disabledControlTextColor",
    "selectedControlColor", "selectedControlTextColor", "alternateSelectedControlTextColor",
    "windowFrameTextColor", "findHighlightColor", "highlightColor", "shadowColor",
    "controlAccentColor", "systemRedColor", "systemOrangeColor", "systemYellowColor",
    "systemGreenColor", "systemMintColor", "systemTealColor", "systemCyanColor",
    "systemBlueColor", "systemIndigoColor", "systemPurpleColor", "systemPinkColor",
    "systemBrownColor", "systemGrayColor",
};

QColor to_qcolor(NSColor *color) {
    NSColor *rgb = [color colorUsingColorSpace:NSColorSpace.sRGBColorSpace];
    if (!rgb)
        return QColor();
    return QColor::fromRgbF(rgb.redComponent, rgb.greenComponent, rgb.blueComponent,
                            rgb.alphaComponent);
}

QString key_for(const char *name) {
    QString key = QString::fromLatin1(name);
    key.chop(5);
    return key;
}

// Dynamic system colours resolve against the drawing appearance: read them
// under the application's current one.
QVariantHash read_colors() {
    __block QVariantHash colors;
    NSAppearance *appearance = NSApp ? NSApp.effectiveAppearance : NSAppearance.currentDrawingAppearance;
    [appearance performAsCurrentDrawingAppearance:^{
        for (const char *name : color_names) {
            SEL selector = NSSelectorFromString([NSString stringWithUTF8String:name]);
            if (![NSColor respondsToSelector:selector])
                continue;
            auto read = reinterpret_cast<NSColor *(*)(id, SEL)>(objc_msgSend);
            colors.insert(key_for(name), to_qcolor(read([NSColor class], selector)));
        }
        NSArray<NSColor *> *rows = NSColor.alternatingContentBackgroundColors;
        for (NSUInteger row = 0; row < rows.count && row < 2; ++row)
            colors.insert(QStringLiteral("alternatingContentBackground%1").arg(row),
                          to_qcolor(rows[row]));
    }];
    return colors;
}

// Qt cannot find SF Mono's hidden family (`.AppleSystemUIFontMonospaced`)
// and substitutes another font, so only an exact match counts. Else Menlo.
QString monospaced_family() {
    NSFont *font = [NSFont monospacedSystemFontOfSize:NSFont.systemFontSize weight:NSFontWeightRegular];
    const QString family = QString::fromNSString(font.familyName);
    if (!family.isEmpty() && QFontInfo(QFont(family)).family() == family)
        return family;
    return QFontDatabase::systemFont(QFontDatabase::FixedFont).family();
}

void read_fonts(QQmlPropertyMap *theme) {
    theme->insert(QStringLiteral("defaultFont"), QFontDatabase::systemFont(QFontDatabase::GeneralFont));
    theme->insert(QStringLiteral("smallFont"), QFontDatabase::systemFont(QFontDatabase::SmallestReadableFont));
    QFont fixed = QFontDatabase::systemFont(QFontDatabase::FixedFont);
    fixed.setFamily(monospaced_family());
    theme->insert(QStringLiteral("fixedFont"), fixed);
}

class SystemTheme : public QQmlPropertyMap {
public:
    SystemTheme() : QQmlPropertyMap(this, nullptr) {
        refresh();
        qApp->installEventFilter(this);
        QObject::connect(QGuiApplication::styleHints(), &QStyleHints::colorSchemeChanged, this,
                         [this] { refresh(); });
    }

    bool eventFilter(QObject *watched, QEvent *event) override {
        if (watched == qApp && (event->type() == QEvent::ApplicationPaletteChange
                                || event->type() == QEvent::ApplicationFontChange))
            refresh();
        return QQmlPropertyMap::eventFilter(watched, event);
    }

private:
    void refresh() {
        insert(read_colors());
        read_fonts(this);
    }
};

bool cover(QWindow *window) {
    auto *cocoa = window->nativeInterface<QNativeInterface::Private::QCocoaWindow>();
    if (!cocoa)
        return false;
    cocoa->manageVisualEffectArea(whole_window, QRect(QPoint(), window->size()),
                                  static_cast<NSVisualEffectMaterial>(material_sidebar),
                                  static_cast<NSVisualEffectBlendingMode>(blending_behind_window),
                                  static_cast<NSVisualEffectState>(state_follows_window));
    return true;
}

// Public AppKit only, so the macOS 14 SDK builds it.
void unifyTitlebar(QWindow *window) {
    auto *view = (__bridge NSView *)reinterpret_cast<void *>(window->winId());
    if (!view)
        return;
    NSWindow *nsWindow = view.window;
    if (!nsWindow)
        return;
    nsWindow.titlebarAppearsTransparent = YES;
    nsWindow.styleMask |= NSWindowStyleMaskFullSizeContentView;
    nsWindow.titleVisibility = NSWindowTitleHidden;
    nsWindow.tabbingMode = NSWindowTabbingModeAutomatic;
}

// Whether the OS offers its glass view. The window keeps the sidebar
// material either way, newer macOS renders it in the glass style.
extern "C" bool drakeflake_has_glass() {
    return NSClassFromString(@"NSGlassEffectView") != nullptr;
}

void read_accessibility(QWindow *window) {
    NSWorkspace *workspace = NSWorkspace.sharedWorkspace;
    window->setProperty("reduceTransparency",
                        workspace.accessibilityDisplayShouldReduceTransparency ? true : false);
    window->setProperty("reduceMotion",
                        workspace.accessibilityDisplayShouldReduceMotion ? true : false);
}

// Solid backgrounds hide the effect layer, so switching back only re-covers.
void blur(QWindow *window);

void watch_accessibility(QWindow *window) {
    [[NSWorkspace.sharedWorkspace notificationCenter]
        addObserverForName:NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification
                        object:nil
                         queue:NSOperationQueue.mainQueue
                    usingBlock:^(NSNotification *) {
                        read_accessibility(window);
                        if (window->property("reduceTransparency").toBool())
                            window->setProperty("vibrancy", false);
                        else
                            blur(window);
                    }];
}

void blur(QWindow *window) {
    read_accessibility(window);
    if (window->property("reduceTransparency").toBool())
        return;
    if (!cover(window)) {
        qWarning("no native window for the blur, the window stays opaque");
        return;
    }
    unifyTitlebar(window);
    // QML turns its backgrounds see-through only once the blur is in place.
    window->setProperty("vibrancy", true);
    QObject::connect(window, &QWindow::widthChanged, window, [window] { cover(window); });
    QObject::connect(window, &QWindow::heightChanged, window, [window] { cover(window); });
}

} // namespace

// Before the application exists: the windows' backgrounds need an alpha
// channel for the blur to show through.
extern "C" void drakeflake_window_alpha() {
    QSurfaceFormat format = QSurfaceFormat::defaultFormat();
    format.setAlphaBufferSize(8);
    QSurfaceFormat::setDefaultFormat(format);
}

extern "C" void drakeflake_register_system_theme() {
    // The QML asks for "monospace", a family macOS does not have.
    const QString fixed = monospaced_family();
    QFont::insertSubstitution(QStringLiteral("monospace"), fixed);

    auto *theme = new SystemTheme();
    theme->setParent(qApp);
    qDebug("system theme: %d colours from macOS, accent %s, fonts %s and %s",
           int(theme->count()) - 3,
           qPrintable(theme->value(QStringLiteral("controlAccent")).value<QColor>().name()),
           qPrintable(QFontDatabase::systemFont(QFontDatabase::GeneralFont).family()),
           qPrintable(fixed));
    qmlRegisterSingletonInstance<QQmlPropertyMap>("DrakeFlake.Native", 1, 0, "SystemTheme", theme);
}

// Queued: the QML window has its native window once the event loop runs.
extern "C" void drakeflake_blur_windows() {
    QMetaObject::invokeMethod(
        qApp,
        [] {
            if (QGuiApplication::platformName() != QLatin1String("cocoa"))
                return;
            for (QWindow *window : QGuiApplication::topLevelWindows()) {
                if (window->property("vibrancy").isValid()) {
                    watch_accessibility(window);
                    blur(window);
                }
            }
        },
        Qt::QueuedConnection);
}

extern "C" void drakeflake_set_dock_badge(int count) {
    if (!qApp)
        return;
    NSApp.dockTile.badgeLabel = count > 0 ? [@(count) stringValue] : @"";
}

extern "C" void drakeflake_request_attention() {
    if (!qApp)
        return;
    [NSApp requestUserAttention:NSCriticalRequest];
}

extern "C" void drakeflake_set_window_dirty(bool dirty) {
    if (!qApp || QGuiApplication::platformName() != QLatin1String("cocoa"))
        return;
    for (QWindow *window : QGuiApplication::topLevelWindows()) {
        if (!window->property("vibrancy").isValid())
            continue;
        auto *view = (__bridge NSView *)reinterpret_cast<void *>(window->winId());
        NSWindow *nsWindow = view ? view.window : nil;
        if (nsWindow)
            nsWindow.documentEdited = dirty ? YES : NO;
    }
}

// Any role counts: `open -t` only asks for the editor role and lands in
// TextEdit when the user picked their editor as a viewer.
extern "C" bool drakeflake_default_text_app(char *path, size_t path_size, char *bundle,
                                            size_t bundle_size) {
    @autoreleasepool {
        UTType *text = [UTType typeWithIdentifier:@"public.plain-text"];
        NSURL *app = [NSWorkspace.sharedWorkspace URLForApplicationToOpenContentType:text];
        if (app == nil)
            return false;
        NSString *identifier = [NSBundle bundleWithURL:app].bundleIdentifier ?: @"";
        return strlcpy(path, app.fileSystemRepresentation, path_size) < path_size
            && strlcpy(bundle, identifier.UTF8String, bundle_size) < bundle_size;
    }
}

// Drawn in the appearance's label colour, like Breeze's light and dark sets,
// because Kirigami only recolours mask icons.
extern "C" bool drakeflake_render_symbol(const char *symbol, unsigned points, unsigned scale,
                                         bool dark, const char *path) {
    @autoreleasepool {
        NSImage *image = [NSImage imageWithSystemSymbolName:@(symbol) accessibilityDescription:nil];
        if (image == nil)
            return false;

        // Symbols carry their own margins: about 80 % of the square looks
        // as large as a Breeze icon of the same size.
        NSImageSymbolConfiguration *config =
            [NSImageSymbolConfiguration configurationWithPointSize:points * 0.8
                                                            weight:NSFontWeightRegular
                                                             scale:NSImageSymbolScaleMedium];
        NSImage *sized = [image imageWithSymbolConfiguration:config];
        if (sized == nil)
            return false;

        const NSInteger pixels = points * scale;
        NSBitmapImageRep *bitmap =
            [[NSBitmapImageRep alloc] initWithBitmapDataPlanes:nullptr
                                                    pixelsWide:pixels
                                                    pixelsHigh:pixels
                                                 bitsPerSample:8
                                               samplesPerPixel:4
                                                      hasAlpha:YES
                                                      isPlanar:NO
                                                colorSpaceName:NSDeviceRGBColorSpace
                                                   bytesPerRow:0
                                                  bitsPerPixel:0];
        if (bitmap == nil)
            return false;
        bitmap.size = NSMakeSize(points, points);

        const NSSize shape = sized.size;
        const CGFloat fit = MIN(points / shape.width, points / shape.height);
        const CGFloat fitted = MIN(fit, 1.0);
        const NSSize drawn = NSMakeSize(shape.width * fitted, shape.height * fitted);
        const NSRect rect = NSMakeRect((points - drawn.width) / 2, (points - drawn.height) / 2,
                                       drawn.width, drawn.height);

        [NSGraphicsContext saveGraphicsState];
        NSGraphicsContext.currentContext =
            [NSGraphicsContext graphicsContextWithBitmapImageRep:bitmap];
        [sized drawInRect:rect
                 fromRect:NSZeroRect
                operation:NSCompositingOperationSourceOver
                 fraction:1.0];
        [(dark ? [NSColor colorWithWhite:1 alpha:0.85] : [NSColor colorWithWhite:0 alpha:0.85])
            set];
        NSRectFillUsingOperation(NSMakeRect(0, 0, points, points),
                                 NSCompositingOperationSourceAtop);
        [NSGraphicsContext restoreGraphicsState];

        NSData *png = [bitmap representationUsingType:NSBitmapImageFileTypePNG properties:@{}];
        return [png writeToFile:@(path) atomically:YES];
    }
}
