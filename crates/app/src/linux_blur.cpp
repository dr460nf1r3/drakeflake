#include <QtCore/QEvent>
#include <QtCore/QMetaObject>
#include <QtCore/QObject>
#include <QtCore/QTimer>
#include <QtCore/QVariant>
#include <QtGui/QGuiApplication>
#include <QtGui/QSurfaceFormat>
#include <QtGui/QWindow>
#include <QtGui/qguiapplication_platform.h>
#include <QtGui/qpa/qplatformwindow_p.h>

namespace {
using ApplyFn = bool (*)(void *display, void *surface, bool enabled);
using ForgetFn = void (*)(void *surface);

ApplyFn apply_surface = nullptr;
ForgetFn forget_surface = nullptr;
bool wanted = false;

wl_display *wayland_display() {
    auto *app = qGuiApp->nativeInterface<QNativeInterface::QWaylandApplication>();
    return app ? app->display() : nullptr;
}

bool apply(QWindow *window, bool enabled);

// Qt replaces the wl_surface when a window is hidden and shown again. The
// effect belongs to the old surface, so it is dropped and set up anew.
void track(QWindow *window, QNativeInterface::Private::QWaylandWindow *native) {
    if (window->property("drakeflakeTracked").toBool())
        return;
    window->setProperty("drakeflakeTracked", true);

    QObject::connect(native, &QNativeInterface::Private::QWaylandWindow::surfaceDestroyed, window, [window] {
        const auto surface = window->property("drakeflakeSurface").value<quintptr>();
        if (surface && forget_surface)
            forget_surface(reinterpret_cast<void *>(surface));
        window->setProperty("drakeflakeSurface", QVariant());
    });
    QObject::connect(native, &QNativeInterface::Private::QWaylandWindow::surfaceCreated, window, [window] {
        window->setProperty("vibrancy", QVariant(apply(window, wanted)));
    });
}

// The whole window blurs: the region covers any size, and the compositor
// clips it. It takes effect with the window's next frame.
bool apply(QWindow *window, bool enabled) {
    wl_display *display = wayland_display();
    auto *native = window->nativeInterface<QNativeInterface::Private::QWaylandWindow>();
    if (!display || !native || !apply_surface)
        return false;

    track(window, native);
    wl_surface *surface = native->surface();
    if (!surface)
        return false;

    window->setProperty("drakeflakeSurface", QVariant::fromValue(reinterpret_cast<quintptr>(surface)));
    const bool ok = apply_surface(display, surface, enabled);
    window->requestUpdate();
    return ok;
}

bool apply_all(bool enabled) {
    wanted = enabled;
    bool active = false;
    for (QWindow *window : QGuiApplication::topLevelWindows()) {
        if (!window->property("vibrancy").isValid())
            continue;
        const bool ok = apply(window, enabled);
        window->setProperty("vibrancy", QVariant(ok));
        active = active || ok;
    }
    return active;
}

} // namespace

// Before the application exists: the window background needs an alpha
// channel for the blur to show through.
extern "C" void drakeflake_window_alpha(ApplyFn apply, ForgetFn forget) {
    apply_surface = apply;
    forget_surface = forget;
    QSurfaceFormat format = QSurfaceFormat::defaultFormat();
    format.setAlphaBufferSize(8);
    QSurfaceFormat::setDefaultFormat(format);
}

extern "C" bool drakeflake_blur_apply(bool enabled, bool silent) {
    const bool active = apply_all(enabled);
    if (enabled && !active && !silent)
        qWarning("the compositor offers no blur (ext-background-effect-v1), the window stays opaque");
    return active;
}

// Fallback while the compositor handshake is not ready at show time. Short
// on purpose: each missed interval paints solid before the blur flips the
// window see-through.
constexpr int retry_attempts = 100;
constexpr int retry_interval_ms = 50;

// No Q_OBJECT: this file builds without moc.
class BlurStarter : public QObject {
public:
    BlurStarter(bool enabled, bool silent)
        : enabled_(enabled)
        , silent_(silent)
        , left_(retry_attempts) {
        timer_.setInterval(retry_interval_ms);
        connect(&timer_, &QTimer::timeout, this, [this] { tryApply(); });
    }

    void start() {
        qApp->installEventFilter(this);
        timer_.start();
    }

protected:
    bool eventFilter(QObject *watched, QEvent *event) override {
        const QEvent::Type type = event->type();
        if (type == QEvent::Show || type == QEvent::Expose) {
            const auto *window = qobject_cast<QWindow *>(watched);
            if (window && window->property("vibrancy").isValid())
                tryApply();
        }
        return false;
    }

private:
    void tryApply() {
        if (apply_all(enabled_)) {
            finish();
        } else if (--left_ <= 0) {
            if (enabled_ && !silent_)
                qWarning("the compositor offers no blur (ext-background-effect-v1), the window stays opaque");
            finish();
        }
    }

    void finish() {
        timer_.stop();
        qApp->removeEventFilter(this);
        deleteLater();
    }

    const bool enabled_;
    const bool silent_;
    int left_;
    QTimer timer_;
};

// Queued: the QML window gains its native window once the event loop runs,
// which is still too early on a slow start.
extern "C" void drakeflake_blur_windows(bool enabled, bool silent) {
    QMetaObject::invokeMethod(
        qApp,
        [enabled, silent] {
            if (!enabled || apply_all(enabled))
                return;
            auto *starter = new BlurStarter(enabled, silent);
            starter->setParent(qApp);
            starter->start();
        },
        Qt::QueuedConnection);
}
