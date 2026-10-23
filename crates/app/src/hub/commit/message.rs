#[cfg(test)]
mod tests;

use drakeflake_file::PACKAGES_PATH;
use drakeflake_flake::lock::LockedInput;
use drakeflake_plan::ManagedState;
use drakeflake_render::OptionName;

pub const COMMIT_SUBJECT: &str = "drakeflake: update config";

const MAX_BULLETS: usize = 20;

#[must_use = "the message is only built, nothing is committed"]
pub fn commit_message(
    previous: Option<&ManagedState>,
    plan: &ManagedState,
    old_lock: Option<&[LockedInput]>,
    new_lock: &[LockedInput],
) -> (String, String) {
    let mut bullets = option_bullets(previous, plan);
    bullets.extend(lock_bullets(old_lock, new_lock));
    (COMMIT_SUBJECT.to_owned(), bullets.join("\n"))
}

fn option_bullets(previous: Option<&ManagedState>, plan: &ManagedState) -> Vec<String> {
    let Some(before) = previous else {
        return Vec::new();
    };

    let mut names: Vec<&OptionName> = before.options.keys().chain(plan.options.keys()).collect();
    names.sort();
    names.dedup();

    let mut bullets: Vec<String> = names
        .into_iter()
        .filter_map(|name| {
            match (
                before.options.contains_key(name),
                plan.options.contains_key(name),
            ) {
                (true, false) => Some(format!("- {name} removed")),
                (false, true) => Some(format!("- {name} added")),
                (true, true) if before.options.get(name) != plan.options.get(name) => {
                    Some(format!("- {name} changed"))
                }
                _ => None,
            }
        })
        .collect();

    if before.packages != plan.packages {
        let change = if before.packages.is_empty() || plan.packages.is_empty() {
            if plan.packages.is_empty() {
                "removed"
            } else {
                "added"
            }
        } else {
            "changed"
        };

        bullets.push(format!("- {PACKAGES_PATH} {change}"));
        bullets.sort();
    }

    cap(bullets)
}

fn lock_bullets(old_lock: Option<&[LockedInput]>, new_lock: &[LockedInput]) -> Vec<String> {
    let Some(before) = old_lock else {
        return Vec::new();
    };

    let locked = |input: &LockedInput| (input.rev.clone(), input.locked_url.clone());
    let mut bullets = Vec::new();

    for input in new_lock {
        match before.iter().find(|old| old.name == input.name) {
            None => bullets.push(format!("- {} added", input.name)),
            Some(old) if locked(old) != locked(input) => bullets.push(format!(
                "- {}: {} -> {}",
                input.name,
                describe_locked(old),
                describe_locked(input)
            )),
            Some(_) => {}
        }
    }

    for old in before {
        if !new_lock.iter().any(|input| input.name == old.name) {
            bullets.push(format!("- {} removed", old.name));
        }
    }

    bullets.sort();
    cap(bullets)
}

fn describe_locked(input: &LockedInput) -> String {
    if let Some(rev) = input.rev.as_deref() {
        return rev.chars().take(7).collect();
    }

    if let Some(url) = input.locked_url.as_deref() {
        return url.to_owned();
    }

    input.source.clone()
}

fn cap(mut bullets: Vec<String>) -> Vec<String> {
    if bullets.len() > MAX_BULLETS {
        let more = bullets.len() - MAX_BULLETS;
        bullets.truncate(MAX_BULLETS);
        bullets.push(format!("- … and {more} more changes"));
    }

    bullets
}
