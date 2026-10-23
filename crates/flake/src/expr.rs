use crate::tree::OptionTree;

/// Bump whenever an evaluation expression changes, so cached results from an
/// older app version are not reused.
pub const EVAL_VERSION: u32 = 4;

pub const UNDOCUMENTED: &str = "(This option's documentation could not be evaluated.)";

pub const SIMPLIFY: &str = include_str!("expr/simplify.nix");

/// Every node's key is its full dotted path, so an uncatchable evaluation error names the broken option.
pub const WALK: &str = include_str!("expr/walk.nix");

pub const SHARD: &str = include_str!("expr/shard.nix");

pub const USER_FILES: &str = include_str!("expr/user-files.nix");

pub const SNAPSHOT: &str = include_str!("expr/snapshot.nix");

pub const SELECTED: &str = include_str!("expr/selected.nix");

/// `check` of `listOf`/`attrsOf` only tests the container, so element types are checked recursively.
pub const CHECK: &str = include_str!("expr/check.nix");

pub const VALUE: &str = include_str!("expr/value.nix");

pub const VALUES: &str = include_str!("expr/values.nix");

pub const PACKAGES: &str = include_str!("expr/packages.nix");

pub const PACKAGE_SETS: &str = include_str!("expr/package-sets.nix");

pub const PACKAGE_SET: &str = include_str!("expr/package-set.nix");

pub const PACKAGE_META: &str = include_str!("expr/package-meta.nix");

pub const HOME_MANAGER_USERS: &str = include_str!("expr/home-manager-users.nix");

pub const PERSISTENCE: &str = include_str!("expr/persistence.nix");

pub const INSTALLED: &str = include_str!("expr/installed.nix");

pub const OPTIONS_DOC: &str = include_str!("expr/options-doc.nix");

/// Forces the manual on over every module, so the option index ignores the user's docs settings.
pub const MANUAL: &str = include_str!("expr/manual.nix");

#[must_use = "the template is not changed in place"]
pub fn resolve(template: &str) -> String {
    // Fragments first: they contain the placeholders below.
    template
        .replace("__SIMPLIFY__", SIMPLIFY)
        .replace("__WALK__", WALK)
        .replace("__USER_FILES__", USER_FILES)
        .replace("__SHARD__", SHARD)
        .replace("__TREE__", &OptionTree::System.options_expr())
        .replace("__CONFIG__", &OptionTree::System.config_expr())
        .replace("__DEFAULTS__", "true")
        .replace("__FRAMEWORK_ROOTS__", "[ ]")
}

#[cfg(test)]
mod tests;
