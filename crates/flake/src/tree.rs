use drakeflake_render::{UserName, render_string};
use serde_json::{Map, Value};

use crate::expr::{SIMPLIFY, USER_FILES, WALK};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptionTree {
    System,
    HomeManager(UserName),
}

impl OptionTree {
    pub(crate) fn options_expr(&self) -> String {
        match self {
            Self::System => "sys.options".to_owned(),
            Self::HomeManager(user) => format!(
                "(let u = sys.options.home-manager.users; n = {}; in if u ? valueMeta.attrs.${{n}}.configuration.options then u.valueMeta.attrs.${{n}}.configuration.options else u.type.getSubOptions [ ])",
                render_string(user)
            ),
        }
    }

    pub(crate) fn config_expr(&self) -> String {
        match self {
            Self::System => "sys.config".to_owned(),
            Self::HomeManager(user) => {
                format!(
                    "(sys.config.home-manager.users.${{{}}})",
                    render_string(user)
                )
            }
        }
    }

    #[must_use = "the prefix is only computed"]
    pub fn prefix(&self) -> String {
        match self {
            Self::System => String::new(),
            Self::HomeManager(user) => format!("home-manager.users.{}.", quote_name(user)),
        }
    }

    fn framework_roots(&self) -> String {
        match self {
            Self::System => "[ ]".to_owned(),
            Self::HomeManager(_) => format!(
                "(let decl = builtins.tryEval (toString (builtins.head (({}).home.stateVersion.declarations or [ \"\" ]))); m = if decl.success then builtins.match \"(.*/)modules/.*\" decl.value else null; in if m == null then [ ] else m)",
                self.options_expr()
            ),
        }
    }

    pub(crate) fn specialise(&self, template: &str) -> String {
        // Outside a real user configuration, Home Manager defaults that read
        // `config` fail uncatchably. Show only their declared `defaultText`.
        let defaults = match self {
            Self::System => "true".to_owned(),
            Self::HomeManager(user) => format!(
                "(sys.options.home-manager.users ? valueMeta.attrs.${{{}}}.configuration.options)",
                render_string(user)
            ),
        };

        // Fragments first: they contain the placeholders below.
        template
            .replace("__SIMPLIFY__", SIMPLIFY)
            .replace("__USER_FILES__", USER_FILES)
            .replace("__WALK__", WALK)
            .replace("__TREE__", &self.options_expr())
            .replace("__CONFIG__", &self.config_expr())
            .replace("__DEFAULTS__", &defaults)
            .replace("__FRAMEWORK_ROOTS__", &self.framework_roots())
    }
}

fn quote_name(name: &str) -> String {
    let plain = name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '\''))
        && name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphabetic() || ch == '_');

    if plain {
        name.to_owned()
    } else {
        format!("\"{}\"", name.replace('"', "\\\""))
    }
}

#[must_use = "the map is consumed and only the result holds it"]
pub fn prefix_keys(map: Map<String, Value>, prefix: &str) -> String {
    if prefix.is_empty() {
        return Value::Object(map).to_string();
    }

    let renamed: Map<String, Value> = map
        .into_iter()
        .map(|(name, value)| {
            if name.starts_with(prefix) {
                (name, value)
            } else {
                (format!("{prefix}{name}"), value)
            }
        })
        .collect();

    Value::Object(renamed).to_string()
}

#[must_use = "the value is consumed and only the result holds it"]
pub fn flatten_options(value: Value) -> Map<String, Value> {
    fn visit(value: Value, out: &mut Map<String, Value>) {
        let Value::Object(mut map) = value else {
            return;
        };

        let sub = map.remove("_drakeflakeSub");

        if let Some(Value::String(name)) = map.remove("_drakeflakeOption") {
            out.insert(name, Value::Object(map));
        } else {
            for child in map.into_values() {
                visit(child, out);
            }
        }

        if let Some(sub) = sub {
            visit(sub, out);
        }
    }

    let mut out = Map::new();
    visit(value, &mut out);
    out
}

#[cfg(test)]
mod tests;
