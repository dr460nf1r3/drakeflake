#[cfg(test)]
mod tests;

use std::borrow::Cow;

use drakeflake_render::render_attr_name;

use super::{PersistEntry, PersistKind, PersistScope, Persistence};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistPlace {
    pub scope: PersistScope,
    pub user: String,
    pub storage: String,
    pub kind: PersistKind,
}

impl PersistPlace {
    #[must_use = "only derives the place"]
    pub fn of(entry: &PersistEntry) -> Self {
        Self {
            scope: entry.scope,
            user: entry.user.clone(),
            storage: entry.storage.clone(),
            kind: entry.kind,
        }
    }

    #[must_use = "only derives the name"]
    pub fn option(&self) -> String {
        let list = match self.kind {
            PersistKind::Dir => "directories",
            PersistKind::File => "files",
        };

        let storage = render_attr_name(&self.storage);
        let user = render_attr_name(&self.user);

        match self.scope {
            PersistScope::System => format!("environment.persistence.{storage}.{list}"),
            PersistScope::User => format!("environment.persistence.{storage}.users.{user}.{list}"),
            PersistScope::Home => {
                format!("home-manager.users.{user}.home.persistence.{storage}.{list}")
            }
        }
    }
}

impl Persistence {
    #[must_use = "only builds the places"]
    pub fn places(&self) -> Vec<PersistPlace> {
        let mut places = Vec::new();

        for storage in &self.storages {
            for kind in [PersistKind::Dir, PersistKind::File] {
                let place = |scope: PersistScope, user: &str| PersistPlace {
                    scope,
                    user: user.to_owned(),
                    storage: storage.clone(),
                    kind,
                };

                places.push(place(PersistScope::System, ""));

                for user in self.homes.keys() {
                    places.push(place(PersistScope::User, user));
                    places.push(place(PersistScope::Home, user));
                }
            }
        }

        places
    }

    #[must_use = "only builds the path"]
    pub fn absolute(&self, place: &PersistPlace, entry: &str) -> String {
        match place.scope {
            PersistScope::System => entry.to_owned(),
            PersistScope::User | PersistScope::Home => {
                let home = self.homes.get(&place.user).map_or_else(
                    || Cow::Owned(format!("/home/{}", place.user)),
                    |home| Cow::Borrowed(home.as_str()),
                );

                format!(
                    "{}/{}",
                    home.trim_end_matches('/'),
                    entry.trim_start_matches('/')
                )
            }
        }
    }

    #[must_use = "only looks the file up"]
    pub fn source_of(&self, entry: &PersistEntry) -> String {
        self.sources
            .iter()
            .find(|source| {
                source.scope == entry.scope
                    && source.user == entry.user
                    && source.storage == entry.storage
                    && source.entry == entry.entry
            })
            .map(|source| flake_relative(&source.file))
            .unwrap_or_default()
    }

    pub fn new_place(
        &self,
        home: Option<(&str, &str)>,
        storage: &str,
        path: &str,
        kind: PersistKind,
    ) -> Result<(PersistPlace, String), String> {
        if path.chars().any(char::is_control) || !path.starts_with('/') {
            return Err(format!("\"{}\" cannot be persisted", path.escape_debug()));
        }

        if storage.is_empty() {
            return Err("No storage location is selected".to_owned());
        }

        let Some((user, folder)) = home else {
            let place = PersistPlace {
                scope: PersistScope::System,
                user: String::new(),
                storage: storage.to_owned(),
                kind,
            };

            return Ok((place, path.to_owned()));
        };

        let entry = path
            .strip_prefix(folder.trim_end_matches('/'))
            .and_then(|rest| rest.strip_prefix('/'))
            .filter(|rest| !rest.is_empty())
            .ok_or_else(|| format!("{path} is not in {folder}"))?;

        let home_module = self.home_module.get(user).copied().unwrap_or(false);

        let place = PersistPlace {
            scope: if home_module {
                PersistScope::Home
            } else {
                PersistScope::User
            },
            user: user.to_owned(),
            storage: storage.to_owned(),
            kind,
        };

        Ok((place, entry.to_owned()))
    }
}

#[must_use = "only builds the path"]
pub fn flake_relative(file: &str) -> String {
    file.strip_prefix("/nix/store/")
        .and_then(|rest| rest.split_once('/'))
        .map_or_else(|| file.to_owned(), |(_, inside)| inside.to_owned())
}
