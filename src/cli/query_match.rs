#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryKey {
    folded: String,
    compact: String,
}

impl QueryKey {
    pub fn new(input: &str) -> Self {
        let folded = input
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let compact = folded.chars().filter(|ch| ch.is_alphanumeric()).collect();
        Self { folded, compact }
    }

    pub fn folded(&self) -> &str {
        &self.folded
    }

    pub fn compact(&self) -> &str {
        &self.compact
    }

    pub fn contains_in(&self, haystack: &str) -> bool {
        if self.folded.is_empty() {
            return false;
        }

        let hay = QueryKey::new(haystack);
        hay.folded.contains(&self.folded)
            || (!self.compact.is_empty() && hay.compact.contains(&self.compact))
    }
}

pub fn title_match_type(title: &str, id: &str, query: &str) -> Option<(&'static str, u8)> {
    if title == query {
        return Some(("exact", 0));
    }

    let title_lc = title.to_lowercase();
    let query_lc = query.to_lowercase();
    if title_lc == query_lc {
        return Some(("exact_casefold", 1));
    }

    let title_key = QueryKey::new(title);
    let query_key = QueryKey::new(query);
    if query_key.folded().is_empty() {
        return None;
    }

    if title_key.folded() == query_key.folded() {
        return Some(("exact_normalized", 2));
    }
    if !query_key.compact().is_empty() && title_key.compact() == query_key.compact() {
        return Some(("exact_compact", 3));
    }
    if title_key.folded().starts_with(query_key.folded()) {
        return Some(("prefix", 4));
    }
    if !query_key.compact().is_empty() && title_key.compact().starts_with(query_key.compact()) {
        return Some(("prefix_compact", 5));
    }
    if title_key.folded().contains(query_key.folded()) {
        return Some(("contains", 6));
    }
    if !query_key.compact().is_empty() && title_key.compact().contains(query_key.compact()) {
        return Some(("contains_compact", 7));
    }
    if id.contains(query) {
        return Some(("id_contains", 8));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_key_folds_whitespace_and_case() {
        let key = QueryKey::new("  Week   1  ");
        assert_eq!(key.folded(), "week 1");
        assert_eq!(key.compact(), "week1");
    }

    #[test]
    fn title_match_recognizes_compact_equivalence() {
        assert_eq!(
            title_match_type("Week1 课程说明", "abc", "Week 1"),
            Some(("prefix_compact", 5))
        );
    }

    #[test]
    fn title_match_prefers_exact_normalized_before_contains() {
        assert_eq!(
            title_match_type("Week   1", "abc", "week 1"),
            Some(("exact_normalized", 2))
        );
    }

    #[test]
    fn contains_in_checks_folded_and_compact_forms() {
        let key = QueryKey::new("Week 1");
        assert!(key.contains_in("课程安排 Week1 导论"));
        assert!(key.contains_in("课程安排 week 1 导论"));
        assert!(!key.contains_in("Week 2"));
    }
}
