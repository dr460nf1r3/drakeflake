use drakeflake_config::{LayoutConfig, LayoutMode, LayoutRule};
use drakeflake_render::split_option_name;

pub const PACKAGES_PATH: &str = "environment.systemPackages";

#[must_use = "only looks up the target file"]
pub fn file_for(layout: &LayoutConfig, option: &str) -> Option<String> {
    if let Some(rule) = matching_rule(layout, option) {
        return Some(rule.file().to_owned());
    }

    let parts = split_option_name(option);

    if let [area, users, user, rest @ ..] = parts.as_slice()
        && area == "home-manager"
        && users == "users"
        && layout.mode != LayoutMode::Single
    {
        let folder = format!("home-manager/{}", segment(user));
        return Some(file_for_parts(layout.mode, rest).map_or_else(
            || format!("{folder}.nix"),
            |file| format!("{folder}/{file}"),
        ));
    }

    file_for_parts(layout.mode, &parts)
}

#[must_use = "only looks up the target file"]
pub fn packages_file(layout: &LayoutConfig) -> Option<String> {
    if let Some(rule) = matching_rule(layout, PACKAGES_PATH) {
        return Some(rule.file().to_owned());
    }

    (layout.mode != LayoutMode::Single).then(|| "packages.nix".to_owned())
}

fn matching_rule<'a>(layout: &'a LayoutConfig, option: &str) -> Option<&'a LayoutRule> {
    layout
        .rules
        .iter()
        .filter(|rule| {
            option == rule.prefix()
                || option
                    .strip_prefix(rule.prefix())
                    .is_some_and(|rest| rest.starts_with('.'))
        })
        .max_by_key(|rule| rule.prefix().len())
}

fn file_for_parts(mode: LayoutMode, parts: &[String]) -> Option<String> {
    match (mode, parts) {
        (LayoutMode::Single, _) | (_, []) => None,
        (LayoutMode::Area, [area, ..]) | (LayoutMode::AreaModule, [area] | [area, _]) => {
            Some(format!("{}.nix", segment(area)))
        }
        (LayoutMode::AreaModule, [area, module, _, ..]) => {
            Some(format!("{}/{}.nix", segment(area), segment(module)))
        }
    }
}

fn segment(part: &str) -> String {
    let cleaned: String = part
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect();

    if cleaned.is_empty() {
        "_".to_owned()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests;
