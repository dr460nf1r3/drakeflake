mod blur;
mod crash;
mod environment;
mod global_menu;
mod graphics;
mod hub;
mod macos;
mod models;
mod startup;
mod stderr_log;
mod window_state;
mod wiring;

use std::io::Write as _;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub use crash::crash_log_path;
use hub::Hub;
use models::relay::Relay;
use qtbridge::QApp;
use startup::Startup;
use wiring::Models;

pub const PROGRAM: &str = "drakeflake";

const UI_LOAD_TIMEOUT: Duration = Duration::from_secs(20);

fn qml_file(file: &str) -> PathBuf {
    let path = std::env::var_os("DRAKEFLAKE_QML_DIR").map_or_else(
        || {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../src/qml")
                .join(file)
        },
        |dir| PathBuf::from(dir).join(file),
    );

    path.canonicalize().unwrap_or(path)
}

pub fn fail_startup(message: &str, hint: &str, code: i32) -> ! {
    log::error!("{message}");
    log::logger().flush();

    writeln!(std::io::stderr(), "{PROGRAM}: {message}{hint}").ok();
    #[expect(
        clippy::exit,
        reason = "a failed start ends the process with its own exit code"
    )]
    std::process::exit(code);
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some(graphics::PROBE_ARG) {
        std::process::exit(graphics::run_probe());
    }

    // The default level first, so loading the config is logged too.
    drakeflake_log::init(PROGRAM, drakeflake_log::DEFAULT_LEVEL);
    let (config, config_warning) = drakeflake_config::load();
    environment::start_logging(&config.general.log_level);
    crash::install_panic_hook();
    environment::allow_debugger_attach();
    let style = environment::choose_controls_style();

    let renderer = graphics::choose_renderer(&drakeflake_exec::Context::system());

    environment::allow_qml_file_reads();
    environment::ensure_qt_paths();
    macos::add_kirigami_theme();

    // Its thread reads the environment, which `set_var` above must not race.
    stderr_log::capture();
    environment::register_icon_theme();
    macos::prepare();
    blur::prepare();

    let main_qml = qml_file("Main.qml");

    if !main_qml.is_file() {
        fail_startup(
            &format!("{} not found (set DRAKEFLAKE_QML_DIR)", main_qml.display()),
            "",
            1,
        );
    }

    let startup = Startup::from_env(config);
    let mode = drakeflake_config::blur_env_override().unwrap_or(startup.config.general.blur);
    let blur_request = blur::BlurRequest::from(mode);
    let start = !startup.setup;
    log::info!(
        "renderer {renderer}, controls style {style}, blur {}, QML {}",
        mode.as_str(),
        main_qml.display()
    );

    let hub = Arc::new(Hub::default());
    let relay = Arc::new(Relay::default());

    if let Some(warning) = config_warning {
        hub.log_warn(warning);
    }

    let models = Models::new(&hub, &relay, startup);
    let mut app = QApp::new();

    app.application_name("DrakeFlake");
    models.expose(&mut app);
    macos::register_system_theme();
    app.load_qml_from_file(&format!("file://{}", main_qml.display()));
    models.attach(&hub, &relay, start);

    // A self-test run must not start the tray of the person running it.
    if std::env::var_os("DRAKEFLAKE_SELFTEST_DIR").is_none() {
        models::settings::tray::start_at_launch(|error| hub.log_error(error));
    }

    blur_windows(mode, blur_request);

    // QApp does not report load failures. If Main.qml never says it is ready,
    // exit with an error instead of running without a window.
    hub.watch_ui(UI_LOAD_TIMEOUT);

    let code = app.run();
    log::info!("event loop ended (exit code {code})");

    // Background threads may still run nix and must not touch Qt objects while those are torn down.
    hub.shutdown();
    relay.detach();
    drop(app);
    drop(models);

    std::process::exit(code);
}

fn blur_windows(mode: drakeflake_config::BlurMode, request: blur::BlurRequest) {
    if mode != drakeflake_config::BlurMode::Off {
        macos::window::blur_windows();
    }

    blur::apply_startup(request);
}
