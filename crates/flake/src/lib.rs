pub mod lock;

mod context;
mod discover;
mod error;
mod expr;
mod flake_target;
mod hosts;
mod import_check;
mod index_key;
mod manual;
mod nix;
mod options;
mod packages;
mod parallel;
mod persistence;
mod pool;
mod shard;
mod tree;
mod values;

pub use context::Nix;
pub use discover::find_system_flakes;
pub use error::{FlakeError, FlakeProblem, HostProblem, LockError};
pub use expr::{EVAL_VERSION, UNDOCUMENTED};
pub use flake_target::{
    SystemKind, Target, local_flake_dir, normalize_flake, same_flake, validate_flake, validate_host,
};
pub use hosts::{Host, choose_host, fingerprint, list_hosts, local_hostname};
pub use import_check::{lists_file, module_arg_files};
pub use index_key::{hosts_key, option_index_key};
pub use manual::build_options_json;
pub use nix::{NIX_FEATURES, run_nix};
pub use options::{eval_options_doc, home_manager_users, selected_values, snapshot_values};
pub use packages::{
    installed_packages, package_index, package_meta, package_set_index, package_sets,
};
pub use persistence::{
    PersistEntry, PersistKind, PersistPlace, PersistScope, PersistSource, Persistence,
    flake_relative, persistence,
};
pub use pool::{Priority, eval_jobs, with_eval_slot};
pub use shard::OnSkip;
pub use tree::OptionTree;
pub use values::{TypeCheck, check_value, eval_value, eval_values};

#[cfg(test)]
mod integration;
#[cfg(test)]
mod live;
