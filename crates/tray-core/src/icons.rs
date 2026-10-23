use std::path::PathBuf;

#[must_use = "only locates the folder"]
pub fn icon_dir(relative: &str) -> Option<PathBuf> {
    std::env::var_os("DRAKEFLAKE_ICON_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            let exe = std::env::current_exe().ok()?;
            Some(exe.parent()?.parent()?.join(relative))
        })
}
