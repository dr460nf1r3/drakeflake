#[cfg(test)]
mod tests;

use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Mutex, MutexGuard, PoisonError};

pub fn par_map<T, R>(
    items: Vec<T>,
    threads: usize,
    work: impl Fn(usize, T) -> R + Sync,
    crashed: impl Fn(usize) -> R + Sync,
) -> Vec<R>
where
    T: Send,
    R: Send,
{
    let count = items.len();
    let queue = Mutex::new(items.into_iter().enumerate().collect::<VecDeque<_>>());
    let done = Mutex::new(Vec::with_capacity(count));

    std::thread::scope(|scope| {
        for _ in 0..threads.clamp(1, count.max(1)) {
            scope.spawn(|| {
                loop {
                    let next = locked(&queue).pop_front();
                    let Some((index, item)) = next else {
                        break;
                    };

                    let result = catch_unwind(AssertUnwindSafe(|| work(index, item)))
                        .unwrap_or_else(|_| crashed(index));

                    locked(&done).push((index, result));
                }
            });
        }
    });

    let mut done = done.into_inner().unwrap_or_else(PoisonError::into_inner);
    done.sort_by_key(|(index, _)| *index);
    done.into_iter().map(|(_, result)| result).collect()
}

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}
