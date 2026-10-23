use std::sync::Arc;
use std::time::Instant;

use drakeflake_flake::{FlakeError, Nix, Target};
use drakeflake_fs::fnv1a;
use drakeflake_render::UserName;
use drakeflake_store::{OptionIndex, ValueSnapshot};

use super::load::{Loader, log_step_failure, parse_json};
use super::{Cache, Hub, OptionsMsg, SystemMsg, steps};

type TreeEval = fn(
    &Nix,
    &Target,
    &drakeflake_flake::OptionTree,
    &mut Vec<String>,
    drakeflake_flake::OnSkip<'_>,
) -> Result<String, FlakeError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeData {
    Cached,
    Evaluate,
}

pub struct HomeFiles {
    pub user: UserName,
    pub docs: String,
    pub values: String,
    pub docs_key: String,
    pub docs_skip: String,
    pub values_skip: String,
}

impl HomeFiles {
    pub fn of(user: &UserName) -> Self {
        let key = format!("hm-{:016x}", fnv1a(user.as_bytes()));

        Self {
            user: user.clone(),
            docs: format!("{key}-options.json"),
            values: format!("{key}-values.json"),
            docs_key: format!("{key}-options-key"),
            docs_skip: format!("{key}-options-skip.json"),
            values_skip: format!("{key}-values-skip.json"),
        }
    }
}

impl Loader<'_> {
    pub(super) fn load_home(&self, data: HomeData) -> bool {
        let users = match drakeflake_flake::home_manager_users(self.nix, self.target) {
            Ok(Some(users)) if !users.is_empty() => users,
            Ok(_) => {
                self.mail.system.post(SystemMsg::HomeUsers {
                    users: Vec::new(),
                    selected: UserName::default(),
                    error: None,
                });
                return true;
            }
            Err(error) => {
                self.console
                    .warn(format!("Could not check for Home Manager: {error}"));
                self.mail.system.post(SystemMsg::HomeUsers {
                    users: Vec::new(),
                    selected: UserName::default(),
                    error: Some(error.to_string()),
                });
                return false;
            }
        };

        let previous = self.session.home_user();
        let user = pick_home_user(
            &users,
            previous,
            std::env::var("USER").ok().map(UserName::from),
        );

        log::debug!("home manager users {users:?}, selected {user:?}");
        self.session.select_home_user(Some(user.clone()));
        self.mail.system.post(SystemMsg::HomeUsers {
            users,
            selected: user.clone(),
            error: None,
        });

        self.refresh_home(&user, data)
    }

    pub(super) fn refresh_home(&self, user: &UserName, data: HomeData) -> bool {
        let cache = &Cache::for_target(self.target);
        let files = HomeFiles::of(user);
        let started = Instant::now();

        let epoch = self.session.next_home_epoch();

        if !self.post_home_for(user, vec![OptionsMsg::HomeReset { epoch }]) {
            return false;
        }

        let (docs, values) = self.home_data(cache, &files, data);

        if !self.is_current() {
            return false;
        }

        let index = parse_json(docs, OptionIndex::from_json_str);
        let snapshot = parse_json(values, ValueSnapshot::from_json_str);

        match (index, snapshot) {
            (Ok((docs_text, index)), Ok((values_text, snapshot))) => {
                log::info!(
                    "home manager options of {user} loaded ({} values) in {:.1?}",
                    snapshot.len(),
                    started.elapsed()
                );
                cache.write(&files.docs, &docs_text);
                cache.write(&files.values, &values_text);
                cache.save_text(&files.docs_key, self.session.index_key().as_deref());
                self.post_home_for(
                    user,
                    vec![
                        OptionsMsg::Index(Arc::new(index)),
                        OptionsMsg::Snapshot(Arc::new(snapshot)),
                    ],
                )
            }
            (Err(error), _) | (_, Err(error)) => {
                log_step_failure(&format!("home manager options of {user}"), &error);
                self.post_home_for(user, vec![OptionsMsg::IndexError(error.to_string())]);
                false
            }
        }
    }

    fn post_home_for(&self, user: &str, messages: Vec<OptionsMsg>) -> bool {
        let posted = self.session.while_home_user_is(user, || {
            for message in messages {
                self.mail.post_home(message);
            }
        });

        if !posted {
            log::debug!("home manager options of {user} dropped: another user is selected");
        }

        posted
    }

    fn home_data(
        &self,
        cache: &Cache,
        files: &HomeFiles,
        data: HomeData,
    ) -> (Result<String, FlakeError>, Result<String, FlakeError>) {
        let (nix, target) = (self.nix, self.target);

        let user = files.user.as_str();
        let index_key = self.session.index_key();
        let reuse = data == HomeData::Cached;
        let docs_current =
            reuse || (index_key.is_some() && cache.read(&files.docs_key) == index_key);

        let cached_docs = docs_current.then(|| cache.read(&files.docs)).flatten();
        let cached_values = reuse.then(|| cache.read(&files.values)).flatten();
        log::debug!(
            "home manager options of {user}: docs cached {}, values cached {}",
            cached_docs.is_some(),
            cached_values.is_some()
        );

        let (cached_docs, cached_values) = match (cached_docs, cached_values) {
            (Some(docs), Some(values)) => return (Ok(docs), Ok(values)),
            missing => missing,
        };

        let _activity = self
            .console
            .activity("home", format!("Reading Home Manager options of {user}…"));

        let tree = drakeflake_flake::OptionTree::HomeManager(user.into());
        let evaluate = |running: &Nix, cached: Option<String>, skip_file: &str, eval: TreeEval| {
            cached.map_or_else(
                || {
                    self.console
                        .evaluate_skipping(cache, skip_file, None, |skip, on_skip| {
                            eval(running, target, &tree, skip, on_skip)
                        })
                        .0
                },
                Ok,
            )
        };

        steps::join(
            nix,
            |half| {
                evaluate(
                    half,
                    cached_docs,
                    &files.docs_skip,
                    drakeflake_flake::eval_options_doc,
                )
            },
            || {
                evaluate(
                    nix,
                    cached_values,
                    &files.values_skip,
                    drakeflake_flake::snapshot_values,
                )
            },
        )
    }
}

pub(super) fn select_user(hub: &Arc<Hub>, user: UserName) {
    let generation = hub.session().generation();
    hub.session().select_home_user(Some(user));

    let Some(target) = hub.session().target() else {
        return;
    };

    hub.spawn_background("home-manager", move |hub, nix| {
        Loader::new(hub.services(), nix, generation, &target).load_home(HomeData::Cached);
    });
}

fn pick_home_user(
    users: &[UserName],
    previous: Option<UserName>,
    login: Option<UserName>,
) -> UserName {
    previous
        .filter(|user| users.contains(user))
        .or_else(|| login.filter(|login| users.contains(login)))
        .or_else(|| users.first().cloned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;
