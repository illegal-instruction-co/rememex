use log::debug;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct ScoredResult {
    pub path: String,
    pub snippet: String,
    pub score: f32,
}

pub fn score_results(
    final_results: Vec<(String, String, f32)>,
    used_reranker: bool,
    used_hybrid: bool,
    max_results: usize,
) -> Vec<ScoredResult> {
    let input_count = final_results.len();
    let method = if used_reranker {
        "reranker"
    } else if used_hybrid {
        "hybrid"
    } else {
        "vector"
    };
    let mut scored: Vec<ScoredResult> = if used_reranker {
        final_results
            .into_iter()
            .map(|(path, snippet, raw_score)| {
                let sigmoid = 1.0 / (1.0 + (-raw_score).exp());
                let score = sigmoid * 100.0;
                debug!(
                    "reranker score: raw={:.4} → normalized={:.1} for {}",
                    raw_score, score, path
                );
                ScoredResult {
                    path,
                    snippet,
                    score,
                }
            })
            .collect()
    } else if used_hybrid {
        let max_rrf = final_results.first().map(|(_, _, s)| *s).unwrap_or(1.0);
        final_results
            .into_iter()
            .map(|(path, snippet, rrf_score)| {
                let pct = if max_rrf > 0.0 {
                    (rrf_score / max_rrf) * 100.0
                } else {
                    0.0
                };
                ScoredResult {
                    path,
                    snippet,
                    score: pct,
                }
            })
            .collect()
    } else {
        final_results
            .into_iter()
            .map(|(path, snippet, cosine_dist)| {
                let similarity = (1.0 - cosine_dist).clamp(0.0, 1.0);
                ScoredResult {
                    path,
                    snippet,
                    score: similarity * 100.0,
                }
            })
            .collect()
    };

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    if used_reranker {
        scored.retain(|r| r.score >= 1.0);
    }
    scored.truncate(max_results);
    let score_range = scored.first().map(|f| f.score).unwrap_or(0.0);
    let score_min = scored.last().map(|l| l.score).unwrap_or(0.0);
    debug!(
        "score_results: method={}, input={}, output={}, range={:.1}..{:.1}",
        method,
        input_count,
        scored.len(),
        score_min,
        score_range
    );
    scored
}

fn snippet_similarity(a: &str, b: &str) -> f32 {
    let set_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let set_b: std::collections::HashSet<&str> = b.split_whitespace().collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f32 / union as f32
    }
}

pub fn mmr_select(candidates: Vec<ScoredResult>, k: usize, lambda: f32) -> Vec<ScoredResult> {
    let input_count = candidates.len();
    if candidates.is_empty() || k == 0 {
        return vec![];
    }

    let max_score = candidates[0].score;
    if max_score <= 0.0 {
        return candidates.into_iter().take(k).collect();
    }

    let mut remaining: Vec<(usize, &ScoredResult)> = candidates.iter().enumerate().collect();
    let mut selected: Vec<usize> = Vec::with_capacity(k);

    let first = remaining.remove(0);
    selected.push(first.0);

    while selected.len() < k && !remaining.is_empty() {
        let mut best_idx_in_remaining = 0;
        let mut best_mmr = f32::NEG_INFINITY;

        for (ri, (_ci, candidate)) in remaining.iter().enumerate() {
            let relevance = candidate.score / max_score;

            let max_sim = selected
                .iter()
                .map(|&si| snippet_similarity(&candidate.snippet, &candidates[si].snippet))
                .fold(0.0_f32, f32::max);

            let mmr = lambda * relevance - (1.0 - lambda) * max_sim;

            if mmr > best_mmr {
                best_mmr = mmr;
                best_idx_in_remaining = ri;
            }
        }

        let (ci, _) = remaining.remove(best_idx_in_remaining);
        selected.push(ci);
    }

    let result: Vec<ScoredResult> = selected
        .into_iter()
        .map(|i| candidates[i].clone())
        .collect();
    debug!(
        "mmr_select: input={}, k={}, lambda={:.2}, output={}",
        input_count,
        k,
        lambda,
        result.len()
    );
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mmr_preserves_order_with_lambda_one() {
        let candidates = vec![
            ScoredResult {
                path: "a".into(),
                snippet: "hello world foo".into(),
                score: 90.0,
            },
            ScoredResult {
                path: "b".into(),
                snippet: "hello world bar".into(),
                score: 80.0,
            },
            ScoredResult {
                path: "c".into(),
                snippet: "completely different text".into(),
                score: 70.0,
            },
        ];
        let result = mmr_select(candidates, 3, 1.0);
        assert_eq!(result[0].path, "a");
        assert_eq!(result[1].path, "b");
        assert_eq!(result[2].path, "c");
    }

    #[test]
    fn test_mmr_promotes_diversity() {
        let candidates = vec![
            ScoredResult {
                path: "a".into(),
                snippet: "hello world foo bar baz".into(),
                score: 90.0,
            },
            ScoredResult {
                path: "b".into(),
                snippet: "hello world foo bar qux".into(),
                score: 85.0,
            },
            ScoredResult {
                path: "c".into(),
                snippet: "completely different unique text here".into(),
                score: 70.0,
            },
        ];
        let result = mmr_select(candidates, 3, 0.5);
        assert_eq!(result[0].path, "a");
        assert_eq!(result[1].path, "c");
    }

    #[test]
    fn test_mmr_empty_input() {
        let result = mmr_select(vec![], 5, 0.7);
        assert!(result.is_empty());
    }

    #[test]
    fn test_snippet_similarity_identical() {
        assert!((snippet_similarity("hello world", "hello world") - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_snippet_similarity_disjoint() {
        assert!((snippet_similarity("hello world", "foo bar") - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_snippet_similarity_partial_overlap() {
        let sim = snippet_similarity("hello world foo", "hello bar foo");
        assert!(sim > 0.2 && sim < 0.8, "partial overlap was {}", sim);
    }

    #[test]
    fn test_mmr_k_greater_than_candidates() {
        let candidates = vec![
            ScoredResult {
                path: "a".into(),
                snippet: "one".into(),
                score: 90.0,
            },
            ScoredResult {
                path: "b".into(),
                snippet: "two".into(),
                score: 80.0,
            },
        ];
        let result = mmr_select(candidates, 10, 0.7);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_mmr_single_candidate() {
        let candidates = vec![ScoredResult {
            path: "only".into(),
            snippet: "solo".into(),
            score: 50.0,
        }];
        let result = mmr_select(candidates, 5, 0.7);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].path, "only");
    }

    #[test]
    fn test_mmr_lambda_zero_max_diversity() {
        let candidates = vec![
            ScoredResult {
                path: "a".into(),
                snippet: "hello world foo bar baz".into(),
                score: 95.0,
            },
            ScoredResult {
                path: "b".into(),
                snippet: "hello world foo bar qux".into(),
                score: 90.0,
            },
            ScoredResult {
                path: "c".into(),
                snippet: "completely unique different text here".into(),
                score: 60.0,
            },
        ];
        let result = mmr_select(candidates, 2, 0.0);
        assert_eq!(result[0].path, "a");
        assert_eq!(result[1].path, "c");
    }

    #[test]
    fn test_score_results_hybrid_normalization() {
        let input = vec![
            ("top.rs".into(), "best".into(), 0.032f32),
            ("mid.rs".into(), "medium".into(), 0.016f32),
        ];
        let scored = score_results(input, false, true, 10);
        assert_eq!(scored.len(), 2);
        assert!(
            (scored[0].score - 100.0).abs() < 0.1,
            "top should be 100, got {}",
            scored[0].score
        );
        assert!(
            (scored[1].score - 50.0).abs() < 0.1,
            "mid should be 50, got {}",
            scored[1].score
        );
    }

    #[test]
    fn test_score_results_reranker_sigmoid() {
        let input = vec![
            ("good.rs".into(), "good".into(), 5.0f32),
            ("bad.rs".into(), "bad".into(), -5.0f32),
        ];
        let scored = score_results(input, true, false, 10);
        assert!(
            scored[0].score > 90.0,
            "high raw → high sigmoid: {}",
            scored[0].score
        );
        assert!(
            scored.len() == 1,
            "low sigmoid score should be filtered (< 1.0)"
        );
    }

    #[test]
    fn test_score_results_vector_only() {
        let input = vec![
            ("close.rs".into(), "close".into(), 0.1f32),
            ("far.rs".into(), "far".into(), 0.8f32),
        ];
        let scored = score_results(input, false, false, 10);
        assert_eq!(scored[0].path, "close.rs");
        assert!(scored[0].score > scored[1].score);
        assert!(scored[0].score <= 100.0);
    }

    #[test]
    fn test_score_results_empty() {
        let scored = score_results(vec![], false, true, 10);
        assert!(scored.is_empty());
    }
}
