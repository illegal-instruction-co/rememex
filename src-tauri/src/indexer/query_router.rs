use log::debug;
use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QueryType {
    ExactSymbol,
    Keyword,
    Conceptual,
    ExactMatch,
}

pub struct QueryWeights {
    pub vector_weight: f32,
    pub fts_weight: f32,
    pub use_hyde: bool,
}

static CAMEL_CASE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[a-z][A-Z]").unwrap());

static SNAKE_CASE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[a-zA-Z]_[a-zA-Z]").unwrap());

static CODE_CHARS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[{}\[\]();:=<>|&!]").unwrap());

pub fn classify_query(query: &str) -> QueryType {
    let trimmed = query.trim();

    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        return QueryType::ExactMatch;
    }

    if CAMEL_CASE.is_match(trimmed)
        || SNAKE_CASE.is_match(trimmed)
        || trimmed.contains("::")
        || trimmed.contains(".") && !trimmed.contains(' ')
    {
        return QueryType::ExactSymbol;
    }

    let words: Vec<&str> = trimmed.split_whitespace().collect();

    if words.len() <= 2 && !CODE_CHARS.is_match(trimmed) {
        return QueryType::Keyword;
    }

    QueryType::Conceptual
}

pub fn get_weights(query_type: QueryType) -> QueryWeights {
    match query_type {
        QueryType::ExactMatch => QueryWeights {
            vector_weight: 0.3,
            fts_weight: 1.7,
            use_hyde: false,
        },
        QueryType::ExactSymbol => QueryWeights {
            vector_weight: 0.5,
            fts_weight: 1.5,
            use_hyde: false,
        },
        QueryType::Keyword => QueryWeights {
            vector_weight: 0.8,
            fts_weight: 1.2,
            use_hyde: false,
        },
        QueryType::Conceptual => QueryWeights {
            vector_weight: 1.3,
            fts_weight: 0.7,
            use_hyde: true,
        },
    }
}

pub fn classify_and_weigh(query: &str) -> QueryWeights {
    let query_type = classify_query(query);
    let weights = get_weights(query_type);
    debug!(
        "query_router: {:?} → vector={:.1}, fts={:.1}, hyde={}",
        query_type, weights.vector_weight, weights.fts_weight, weights.use_hyde
    );
    weights
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match_quoted() {
        assert_eq!(
            classify_query("\"connection refused\""),
            QueryType::ExactMatch
        );
    }

    #[test]
    fn test_camel_case_symbol() {
        assert_eq!(classify_query("parseConfig"), QueryType::ExactSymbol);
    }

    #[test]
    fn test_snake_case_symbol() {
        assert_eq!(classify_query("search_pipeline"), QueryType::ExactSymbol);
    }

    #[test]
    fn test_rust_path_symbol() {
        assert_eq!(classify_query("indexer::search"), QueryType::ExactSymbol);
    }

    #[test]
    fn test_dotted_symbol() {
        assert_eq!(
            classify_query("config.embedding_model"),
            QueryType::ExactSymbol
        );
    }

    #[test]
    fn test_short_keyword() {
        assert_eq!(classify_query("database"), QueryType::Keyword);
    }

    #[test]
    fn test_two_word_keyword() {
        assert_eq!(classify_query("file watcher"), QueryType::Keyword);
    }

    #[test]
    fn test_conceptual_query() {
        assert_eq!(
            classify_query("how does the file indexing pipeline work"),
            QueryType::Conceptual
        );
    }

    #[test]
    fn test_conceptual_turkish() {
        assert_eq!(
            classify_query("dosya okuma nasıl çalışıyor"),
            QueryType::Conceptual
        );
    }

    #[test]
    fn test_weights_sum() {
        for qt in [
            QueryType::ExactMatch,
            QueryType::ExactSymbol,
            QueryType::Keyword,
            QueryType::Conceptual,
        ] {
            let w = get_weights(qt);
            let sum = w.vector_weight + w.fts_weight;
            assert!(
                (sum - 2.0).abs() < 0.01,
                "{:?}: weights sum to {}, expected 2.0",
                qt,
                sum
            );
        }
    }

    #[test]
    fn test_empty_query() {
        let qt = classify_query("");
        assert_eq!(qt, QueryType::Keyword);
    }

    #[test]
    fn test_single_char() {
        assert_eq!(classify_query("a"), QueryType::Keyword);
    }

    #[test]
    fn test_three_word_conceptual() {
        assert_eq!(classify_query("search file index"), QueryType::Conceptual);
    }

    #[test]
    fn test_code_chars_in_query() {
        assert_eq!(
            classify_query("if (x == 0) { return; }"),
            QueryType::Conceptual
        );
    }

    #[test]
    fn test_hyde_only_for_conceptual() {
        for qt in [
            QueryType::ExactMatch,
            QueryType::ExactSymbol,
            QueryType::Keyword,
        ] {
            let w = get_weights(qt);
            assert!(!w.use_hyde, "{:?} should not enable HyDE", qt);
        }
        let w = get_weights(QueryType::Conceptual);
        assert!(w.use_hyde, "Conceptual should enable HyDE");
    }
}
