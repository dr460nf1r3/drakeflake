use drakeflake_config::Config;
use drakeflake_render::HostName;

pub struct Startup {
    pub config: Config,
    pub setup: bool,
}

impl Startup {
    pub fn from_env(mut config: Config) -> Self {
        for name in [
            "DRAKEFLAKE_FLAKE",
            "DRAKEFLAKE_HOST",
            "DRAKEFLAKE_MANAGED_FILE",
            "DRAKEFLAKE_BLUR",
        ] {
            if let Some(value) = std::env::var_os(name) {
                log::info!(
                    "{name}={} overrides the saved settings",
                    value.to_string_lossy()
                );
            }
        }

        let nix = &mut config.nix;
        if let Some(flake) = std::env::var("DRAKEFLAKE_FLAKE")
            .ok()
            .filter(|flake| !flake.trim().is_empty())
        {
            nix.flake_path = flake;
        }

        let not_empty = |value: String| (!value.is_empty()).then_some(value);
        nix.host = std::env::var("DRAKEFLAKE_HOST")
            .ok()
            .map(HostName::from)
            .or_else(|| nix.host.take())
            .and_then(|host| (!host.is_empty()).then_some(host));
        nix.managed_path = std::env::var("DRAKEFLAKE_MANAGED_FILE")
            .ok()
            .or_else(|| nix.managed_path.take())
            .and_then(not_empty);

        let setup = needs_setup(&config.nix.flake_path);
        Self { config, setup }
    }
}

fn needs_setup(flake: &str) -> bool {
    if std::env::var("DRAKEFLAKE_SETUP").is_ok_and(|value| value == "1") {
        return true;
    }

    let first_start = !drakeflake_config::config_path().is_ok_and(|path| path.is_file())
        && std::env::var_os("DRAKEFLAKE_FLAKE").is_none();

    let has_flake =
        drakeflake_flake::local_flake_dir(flake).is_some_and(|dir| dir.join("flake.nix").is_file());

    log::debug!("first start: {first_start}, flake at {flake}: {has_flake}");
    first_start && !has_flake
}
