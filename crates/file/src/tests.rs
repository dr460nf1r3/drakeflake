mod files;
mod paths;
mod render;
mod storage;

use std::path::PathBuf;

use drakeflake_plan::{InputMode, ManagedOption, ManagedState, Priority};

fn option(type_name: &str, input: &str) -> ManagedOption {
    ManagedOption::typed(type_name, input)
}

fn sample() -> ManagedState {
    let mut state = ManagedState::default();
    state
        .options
        .insert("b.opt".into(), option("signed integer", "2"));
    state
        .options
        .insert("a.opt".into(), option("boolean", "true"));
    state.options.insert(
        "networking.firewall.allowedTCPPorts".into(),
        option(
            "list of 16 bit unsigned integer; between 0 and 65535 (both inclusive)",
            "22 80",
        ),
    );
    state.packages.insert("vim".into());
    state.packages.insert("kdePackages.kate".into());
    state
}

fn featured() -> ManagedState {
    let mut state = ManagedState::default();
    state
        .options
        .insert("services.openssh.enable".into(), option("boolean", "true"));
    state.options.insert(
        "services.openssh.ports".into(),
        ManagedOption {
            priority: Some(Priority::Before),
            ..option(
                "list of 16 bit unsigned integer; between 0 and 65535 (both inclusive)",
                "2222",
            )
        },
    );
    state.options.insert(
        "networking.domain".into(),
        ManagedOption {
            mode: InputMode::Expression,
            ..option("null or string", "config.networking.hostName + \"-vm\"")
        },
    );
    state.options.insert(
        "services.printing.enable".into(),
        ManagedOption {
            host: Some("dragons-strix".into()),
            priority: Some(Priority::Force),
            ..option("boolean", "true")
        },
    );
    state.options.insert(
        "networking.hostName".into(),
        option("string", "x\"; system.evil = true; \"${builtins.abort 1}"),
    );
    state
}

/// Some programs are missing from the Nix build sandbox.
fn find_in_path(program: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;

    std::env::split_paths(&paths)
        .map(|dir| dir.join(program))
        .find(|candidate| candidate.is_file())
}
