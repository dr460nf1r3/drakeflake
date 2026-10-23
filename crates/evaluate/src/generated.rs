//! The generated files only assign values, so the option index key leaves them out.

use std::path::PathBuf;

use drakeflake_config::LayoutMode;

#[must_use = "the paths are only named, nothing is read"]
pub fn generated_paths(
    flake: &str,
    managed_setting: Option<&str>,
    mode: LayoutMode,
) -> Vec<PathBuf> {
    let local = drakeflake_flake::local_flake_dir(flake);

    drakeflake_file::resolve_managed_path(local.as_deref(), managed_setting, mode).map_or_else(
        |_| Vec::new(),
        |entry| {
            let folder = entry.with_file_name(drakeflake_file::settings_folder(&entry));
            vec![entry, folder]
        },
    )
}
