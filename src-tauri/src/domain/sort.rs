//! Pure sorting helpers. No storage, no SQL — repositories reuse them when they
//! fill `sort_title` / `sort_name` columns.

/// Leading articles dropped from sort keys, longest-first is irrelevant here
/// because every entry ends with a space.
const ARTICLES: [&str; 3] = ["the ", "a ", "an "];

/// Builds the value stored in `tracks.sort_title`, `albums.sort_title` and
/// `artists.sort_name`: lowercased, whitespace collapsed, leading English
/// article removed so "The Beatles" sorts next to "Beatles".
///
/// The article is only stripped when something is left after it, so an artist
/// literally named "The" keeps its key.
pub fn normalize_sort_key(raw: &str) -> String {
    let lowered = raw.to_lowercase();

    let mut collapsed = String::with_capacity(lowered.len());
    for word in lowered.split_whitespace() {
        if !collapsed.is_empty() {
            collapsed.push(' ');
        }
        collapsed.push_str(word);
    }

    for article in ARTICLES {
        if let Some(rest) = collapsed.strip_prefix(article) {
            if !rest.is_empty() {
                return rest.to_owned();
            }
        }
    }

    collapsed
}

#[cfg(test)]
mod tests {
    use super::normalize_sort_key;

    #[test]
    fn lowercases_and_trims() {
        assert_eq!(normalize_sort_key("  Radiohead "), "radiohead");
    }

    #[test]
    fn collapses_inner_whitespace() {
        assert_eq!(
            normalize_sort_key("Pink\t\tFloyd\n Live"),
            "pink floyd live"
        );
    }

    #[test]
    fn strips_leading_articles() {
        assert_eq!(normalize_sort_key("The Beatles"), "beatles");
        assert_eq!(normalize_sort_key("A Perfect Circle"), "perfect circle");
        assert_eq!(normalize_sort_key("An Awesome Wave"), "awesome wave");
        assert_eq!(
            normalize_sort_key("  A   Perfect  Circle "),
            "perfect circle"
        );
    }

    #[test]
    fn strips_only_one_article() {
        assert_eq!(normalize_sort_key("The The Band"), "the band");
    }

    #[test]
    fn keeps_words_that_merely_start_like_an_article() {
        assert_eq!(
            normalize_sort_key("Theatre of Tragedy"),
            "theatre of tragedy"
        );
        assert_eq!(normalize_sort_key("Anathema"), "anathema");
    }

    #[test]
    fn keeps_bare_articles() {
        assert_eq!(normalize_sort_key("The"), "the");
        assert_eq!(normalize_sort_key("A"), "a");
        assert_eq!(normalize_sort_key("The  "), "the");
    }

    #[test]
    fn handles_empty_and_non_latin_input() {
        assert_eq!(normalize_sort_key(""), "");
        assert_eq!(normalize_sort_key("   "), "");
        assert_eq!(normalize_sort_key("Аквариум"), "аквариум");
    }
}
