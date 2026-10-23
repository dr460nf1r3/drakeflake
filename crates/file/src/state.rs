/// Defined in every entry file without effect, so `options._module.args.files` lists exactly the files a host imports.
pub const IMPORT_MARKER: &str = "_module.args = { };";
pub const IMPORT_MARKER_COMMENT: &str =
    "# No effect: lets DrakeFlake check that this file is imported.";
