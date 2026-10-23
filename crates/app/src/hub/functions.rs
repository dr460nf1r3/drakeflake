use std::sync::Arc;
use std::time::Instant;

use drakeflake_functions::{FunctionIndex, index_path};

use super::{FunctionsMsg, Hub};

pub fn load_functions(owner: &Arc<Hub>) {
    owner.spawn_background("functions", |hub, _| {
        let started = Instant::now();
        let message = match index_path() {
            Some(path) if path.exists() => match FunctionIndex::load(&path) {
                Ok(index) => {
                    log::debug!(
                        "function index: {} functions from {} in {:?}",
                        index.len(),
                        path.display(),
                        started.elapsed()
                    );
                    FunctionsMsg::Loaded(Arc::new(index))
                }
                Err(error) => {
                    log::warn!("cannot read the function index: {error:#}");
                    FunctionsMsg::Failed(format!("{error:#}"))
                }
            },
            path => {
                let shown = path
                    .map(|path| path.display().to_string())
                    .unwrap_or_default();

                log::warn!("no function index at {shown:?}: the Functions page stays empty");
                FunctionsMsg::Missing(shown)
            }
        };

        hub.mail.functions.post(message);
    });
}
