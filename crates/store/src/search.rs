// Points per query word, by how well it matches. Only the order of the tiers
// matters: each one must beat any realistic sum of the weaker ones.
const EXACT_NAME: i64 = 1000;
const LAST_SEGMENT: i64 = 400;
const SEGMENT: i64 = 200;
const SEGMENT_PREFIX: i64 = 120;
const NAME_SUBSTRING: i64 = 80;
const DESCRIPTION: i64 = 10;
const DOTTED_PREFIX: i64 = 500;
const LENGTH_PENALTY_CHARS: usize = 6;

#[derive(Debug, Default)]
pub struct Haystacks {
    names: Vec<String>,
    descriptions: Vec<String>,
}

impl Haystacks {
    #[must_use = "lowercasing every entry is costly and has no other effect"]
    pub fn new<'a>(entries: impl Iterator<Item = (&'a str, &'a str)>) -> Self {
        let (names, descriptions) = entries
            .map(|(name, description)| (name.to_lowercase(), description.to_lowercase()))
            .unzip();

        Self {
            names,
            descriptions,
        }
    }

    #[must_use = "the lookup has no other effect"]
    pub fn get(&self, index: usize) -> Option<(&str, &str)> {
        Some((
            self.names.get(index)?.as_str(),
            self.descriptions.get(index)?.as_str(),
        ))
    }
}

const fn is_separator(ch: char) -> bool {
    matches!(ch, '.' | '-' | '_')
}

fn segment_tier(name: &str, word: &str) -> Option<i64> {
    let mut prefix = false;

    for segment in name.split(is_separator) {
        if segment == word {
            return Some(SEGMENT);
        }

        prefix |= segment.starts_with(word);
    }

    prefix.then_some(SEGMENT_PREFIX)
}

fn score(words: &[&str], whole_query: &str, name: &str, description: &str) -> Option<i64> {
    let mut total: i64 = 0;

    for word in words {
        let word_score = if name == *word {
            EXACT_NAME
        } else if name.rsplit('.').next() == Some(word) {
            LAST_SEGMENT
        } else if let Some(tier) = segment_tier(name, word) {
            tier
        } else if name.contains(word) {
            NAME_SUBSTRING
        } else if description.contains(word) {
            DESCRIPTION
        } else {
            return None;
        };

        total += word_score;
    }

    if whole_query.contains('.') && name.starts_with(whole_query) {
        total += DOTTED_PREFIX;
    }

    let length_penalty = i64::try_from(name.len() / LENGTH_PENALTY_CHARS).unwrap_or(i64::MAX);

    Some(total.saturating_sub(length_penalty))
}

#[must_use = "the ranked matches are the only result"]
pub fn rank<'a>(
    query: &str,
    count: usize,
    haystack: impl Fn(usize) -> Option<(&'a str, &'a str)>,
) -> Vec<usize> {
    let lowered = query.trim().to_lowercase();
    let words: Vec<&str> = lowered.split_whitespace().collect();

    if words.is_empty() {
        return (0..count)
            .filter(|index| haystack(*index).is_some())
            .collect();
    }

    let mut scored: Vec<(i64, usize)> = (0..count)
        .filter_map(|index| {
            let (name, description) = haystack(index)?;
            score(&words, &lowered, name, description).map(|points| (points, index))
        })
        .collect();

    scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    scored.into_iter().map(|(_, index)| index).collect()
}
