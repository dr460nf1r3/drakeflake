#[cfg(test)]
mod tests;

const MAX_CELLS: usize = 40_000;

#[derive(Debug, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: usize,
    pub removed: usize,
    pub new_start: usize,
    pub added: usize,
}

struct Lengths {
    cells: Vec<usize>,
    width: usize,
}

impl Lengths {
    fn new(old: &[&str], new: &[String]) -> Self {
        let width = new.len() + 1;
        let mut lengths = Self {
            cells: vec![0; (old.len() + 1) * width],
            width,
        };

        for i in (0..old.len()).rev() {
            for j in (0..new.len()).rev() {
                let length = if same(old, new, i, j) {
                    lengths.at(i + 1, j + 1) + 1
                } else {
                    lengths.at(i + 1, j).max(lengths.at(i, j + 1))
                };

                if let Some(cell) = lengths.cells.get_mut(i * width + j) {
                    *cell = length;
                }
            }
        }

        lengths
    }

    fn at(&self, i: usize, j: usize) -> usize {
        self.cells.get(i * self.width + j).copied().unwrap_or(0)
    }
}

pub fn hunks(old: &[&str], new: &[String]) -> Vec<Hunk> {
    let (prefix, suffix) = common_ends(old, new);
    let old = old.get(prefix..old.len() - suffix).unwrap_or_default();
    let new = new.get(prefix..new.len() - suffix).unwrap_or_default();

    if old.is_empty() && new.is_empty() {
        return Vec::new();
    }

    if old.len().saturating_mul(new.len()) > MAX_CELLS {
        return vec![Hunk {
            old_start: prefix,
            removed: old.len(),
            new_start: prefix,
            added: new.len(),
        }];
    }

    let lengths = Lengths::new(old, new);
    let mut hunks = Vec::new();
    let mut open: Option<Hunk> = None;
    let (mut i, mut j) = (0, 0);

    while i < old.len() || j < new.len() {
        if same(old, new, i, j) {
            hunks.extend(open.take());
            i += 1;
            j += 1;
            continue;
        }

        let hunk = open.get_or_insert(Hunk {
            old_start: prefix + i,
            removed: 0,
            new_start: prefix + j,
            added: 0,
        });

        if j < new.len() && (i == old.len() || lengths.at(i, j + 1) >= lengths.at(i + 1, j)) {
            hunk.added += 1;
            j += 1;
        } else {
            hunk.removed += 1;
            i += 1;
        }
    }

    hunks.extend(open);

    hunks
}

fn common_ends(old: &[&str], new: &[String]) -> (usize, usize) {
    let prefix = old
        .iter()
        .zip(new)
        .take_while(|(old, new)| **old == new.as_str())
        .count();

    let longest = old.len().min(new.len()) - prefix;
    let suffix = old
        .iter()
        .rev()
        .zip(new.iter().rev())
        .take(longest)
        .take_while(|(old, new)| **old == new.as_str())
        .count();

    (prefix, suffix)
}

fn same(old: &[&str], new: &[String], i: usize, j: usize) -> bool {
    old.get(i)
        .zip(new.get(j))
        .is_some_and(|(old, new)| *old == new.as_str())
}
