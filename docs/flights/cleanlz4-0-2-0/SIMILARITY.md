# Contamination score

The subject (crates/cleanlz4/src) was compared with 2 test pairs (subject-vs-indep, subject-vs-c) and placed between a negative control (rust-independent-vs-indep) and a positive control (rust-derived-vs-indep). A test pair at or below the negative control on a metric that separates the controls shows no similarity beyond independent work.

## Positioning

- subject-vs-indep, ngram_jaccard@5: 0.2245 (c = -0.0010, below negative)
- subject-vs-indep, ngram_containment@5: 0.4817 (c = -0.0206, below negative)
- subject-vs-indep, ngram_jaccard@10: 0.0529 (c = 0.0016, at or near negative)
- subject-vs-indep, ngram_containment@10: 0.1328 (c = -0.0356, below negative)
- subject-vs-indep, ident_jaccard: 0.0792 (c = 0.0506, at or near negative)
- subject-vs-indep, ident_match_rate_left: 0.1918 (c = 0.1288, at or near negative)
- subject-vs-indep, comment_jaccard: 0.0007 (c = n/a, not separated)
- subject-vs-c, ngram_jaccard@5: 0.0349 (c = -0.2516, below negative)
- subject-vs-c, ngram_containment@5: 0.1125 (c = -0.7605, below negative)
- subject-vs-c, ngram_jaccard@10: 0.0006 (c = -0.0553, below negative)
- subject-vs-c, ngram_containment@10: 0.0027 (c = -0.1935, below negative)
- subject-vs-c, ident_jaccard: 0.0128 (c = -0.0670, below negative)
- subject-vs-c, ident_match_rate_left: 0.0685 (c = -0.0041, below negative)
- subject-vs-c, comment_jaccard: 0.0001 (c = n/a, not separated)

## Verdict

independence supported on every separable metric

## Raw scores

| pair | role | ngram_jaccard@5 | ngram_containment@5 | ngram_jaccard@10 | ngram_containment@10 | ident_jaccard | ident_match_rate_left | comment_jaccard |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| rust-independent-vs-indep | negative | 0.2253 | 0.4920 | 0.0514 | 0.1622 | 0.0506 | 0.0723 | 0.0001 |
| rust-derived-vs-indep | positive | 0.9817 | 0.9909 | 0.9715 | 0.9859 | 0.6157 | 1.0000 | 0.0000 |
| subject-vs-indep | test | 0.2245 | 0.4817 | 0.0529 | 0.1328 | 0.0792 | 0.1918 | 0.0007 |
| subject-vs-c | test | 0.0349 | 0.1125 | 0.0006 | 0.0027 | 0.0128 | 0.0685 | 0.0001 |

## Limits

- This is a lexical approximation of cleanflate-bench's tree-sitter token method: the same normalization but no parse tree, so a C macro body and a Rust impl-for block are handled by rule rather than by grammar.
- There is no clone detector: copied logic that was renamed and reordered below the n-gram window is not found.
- Cross-language pairs score lower on token metrics by construction, as the bench found: its n-gram metrics did not separate its controls, and its identifier and comment metrics did.
- A structural metric needs a same-language positive control.
- One target library is n = 1: a score is evidence about this subject and this prior art, not a general property.
