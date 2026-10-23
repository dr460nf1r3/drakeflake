use std::time::Instant;

use drakeflake_flake::{Nix, Priority};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Urgency {
    Urgent,
    Normal,
}

pub struct Step<'a> {
    name: &'static str,
    priority: Priority,
    job: Box<dyn FnOnce(&Nix) -> bool + Send + 'a>,
}

impl<'a> Step<'a> {
    pub fn new(
        name: &'static str,
        urgency: Urgency,
        job: impl FnOnce(&Nix) -> bool + Send + 'a,
    ) -> Self {
        Self {
            name,
            priority: match urgency {
                Urgency::Urgent => Priority::urgent(),
                Urgency::Normal => Priority::normal(),
            },
            job: Box::new(job),
        }
    }

    fn run(self, load: &Nix) -> bool {
        let started = Instant::now();
        let ok = (self.job)(&load.with_priority(self.priority));
        log::debug!(
            "load step {} finished in {:.1?} (ok {ok})",
            self.name,
            started.elapsed()
        );
        ok
    }
}

pub fn run(nix: &Nix, steps: Vec<Step<'_>>) -> bool {
    std::thread::scope(|scope| {
        let mut handles = Vec::with_capacity(steps.len());

        for step in steps {
            let name = step.name;
            let spawned = std::thread::Builder::new()
                .name(format!("df-{name}"))
                .spawn_scoped(scope, move || step.run(nix));

            handles.push((name, spawned));
        }

        handles
            .into_iter()
            .map(|(name, spawned)| match spawned {
                Ok(handle) => handle.join().unwrap_or_else(|_| {
                    log::error!("load step {name} panicked");
                    false
                }),
                Err(err) => {
                    log::error!("cannot start load step {name}: {err}");
                    false
                }
            })
            .fold(true, |all, ok| all & ok)
    })
}

pub fn join<A, B>(
    nix: &Nix,
    first: impl FnOnce(&Nix) -> A + Send,
    second: impl FnOnce() -> B,
) -> (A, B)
where
    A: Send,
{
    let sibling = nix.sibling();

    std::thread::scope(|scope| {
        let handle = scope.spawn(move || first(&sibling));

        let other = second();
        let result = handle
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic));

        (result, other)
    })
}

#[cfg(test)]
mod tests;
